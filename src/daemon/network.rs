//! Iroh-based peer-to-peer networking layer of the daemon.

use crate::daemon::{Error, Result};
use p2p_chat::schemas::NetEvent;

use iroh::{
  endpoint::{presets, Connection},
  Endpoint, EndpointAddr, EndpointId, RelayMode, SecretKey,
};
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};
use tokio::sync::{mpsc, Mutex};

// ALPN used by this chat protocol.
const CHAT_ALPN: &[u8] = b"hse-p2pchat/mvp/1";
// Maximum message size accepted from a peer.
const MAX_MESSAGE_BYTES: usize = 64 * 1024;

type PeerMap = Arc<Mutex<HashMap<String, Connection>>>;
type DisconnectSet = Arc<Mutex<HashSet<String>>>;

// Runtime networking facade used by the daemon's business logic.
#[derive(Clone)]
pub(super) struct NetworkHandle {
  endpoint: Endpoint,
  peers: PeerMap,
  disconnecting: DisconnectSet,
  events_tx: mpsc::Sender<NetEvent>,
}

impl NetworkHandle {
  // Creates endpoint, waits until it is online, and starts incoming accept loop.
  pub(super) async fn new() -> Result<(Self, mpsc::Receiver<NetEvent>)> {
    let endpoint = Endpoint::builder(presets::N0)
      .secret_key(SecretKey::generate())
      .alpns(vec![CHAT_ALPN.to_vec()])
      .relay_mode(RelayMode::Default)
      .bind()
      .await
      .map_err(Error::other)?;

    // For tickets we need addressing details (relay/direct addrs), so wait until
    // the endpoint is online and has discovered them.
    endpoint.online().await;

    let (events_tx, events_rx) = mpsc::channel::<NetEvent>(256);
    let handle = Self {
      endpoint: endpoint.clone(),
      peers: Arc::new(Mutex::new(HashMap::new())),
      disconnecting: Arc::new(Mutex::new(HashSet::new())),
      events_tx: events_tx.clone(),
    };

    // Accept incoming connections in background for the whole daemon lifetime.
    tokio::spawn(run_accept_loop(
      endpoint,
      handle.peers.clone(),
      handle.disconnecting.clone(),
      events_tx,
    ));

    Ok((handle, events_rx))
  }

  // Returns a shareable connection ticket.
  pub(super) fn my_ticket(&self) -> Result<String> {
    serde_json::to_string(&self.endpoint.addr()).map_err(Error::other)
  }

  // Connects to a peer by:
  // - full ticket (JSON-serialized `EndpointAddr`) or
  // - bare endpoint id (`EndpointId` string).
  pub(super) async fn connect(&self, ticket: &str) -> Result<String> {
    let addr = parse_connect_target(ticket)?;
    let conn = self
      .endpoint
      .connect(addr, CHAT_ALPN)
      .await
      .map_err(Error::other)?;
    let peer_id = conn.remote_id().to_string();
    register_connection(
      self.peers.clone(),
      self.disconnecting.clone(),
      self.events_tx.clone(),
      conn,
      peer_id.clone(),
      false,
    )
    .await;
    Ok(peer_id)
  }

  // Disconnects from known peer (best effort) and emits disconnect event.
  pub(super) async fn disconnect(&self, peer_id: &str) -> Result<bool> {
    let mut peers = self.peers.lock().await;
    let conn = peers.remove(peer_id);
    drop(peers);

    if let Some(conn) = conn {
      self.disconnecting.lock().await.insert(peer_id.to_owned());
      conn.close(0u8.into(), b"client requested disconnect");
      Ok(true)
    } else {
      Ok(false)
    }
  }

  // Sends one text message over a fresh bi-directional stream on existing connection.
  pub(super) async fn send_message(
    &self,
    peer_id: &str,
    message: &str,
  ) -> Result<()> {
    let conn = {
      let peers = self.peers.lock().await;
      peers.get(peer_id).cloned()
    }
    .ok_or_else(|| Error::other(format!("peer {peer_id} is not connected")))?;

    let (mut send, _) = conn.open_bi().await.map_err(Error::other)?;
    send
      .write_all(message.as_bytes())
      .await
      .map_err(Error::other)?;
    send.finish().map_err(Error::other)?;
    Ok(())
  }

  // Lists currently connected peers.
  pub(super) async fn list_peers(&self) -> Vec<String> {
    let peers = self.peers.lock().await;
    peers.keys().cloned().collect()
  }
}

// Best-effort parser for user-provided connect target.
//
// Accepted formats:
// 1. Exact JSON ticket returned by `myid`.
// 2. A copied REPL line like `my ticket: { ...json... }`.
// 3. Bare endpoint id string.
fn parse_connect_target(raw: &str) -> Result<EndpointAddr> {
  let trimmed = raw.trim();
  let payload = trimmed
    .strip_prefix("my ticket:")
    .map(str::trim)
    .unwrap_or(trimmed);

  if let Ok(addr) = serde_json::from_str::<EndpointAddr>(payload) {
    return Ok(addr);
  }

  let endpoint_id = payload.parse::<EndpointId>().map_err(|_| {
    Error::other("connect expects either a full ticket JSON or a valid endpoint id")
  })?;
  Ok(EndpointAddr::from(endpoint_id))
}

async fn run_accept_loop(
  endpoint: Endpoint,
  peers: PeerMap,
  disconnecting: DisconnectSet,
  events_tx: mpsc::Sender<NetEvent>,
) {
  while let Some(incoming) = endpoint.accept().await {
    let accepting = match incoming.accept() {
      Ok(accepting) => accepting,
      Err(err) => {
        log::warn!("Failed to accept incoming connection: {err}");
        continue;
      }
    };

    match accepting.await {
      Ok(conn) => {
        let peer_id = conn.remote_id().to_string();
        register_connection(
          peers.clone(),
          disconnecting.clone(),
          events_tx.clone(),
          conn,
          peer_id,
          true,
        )
        .await;
      }
      Err(err) => {
        log::warn!("Incoming connection handshake failed: {err}");
      }
    }
  }
}

// Stores the connection and starts stream-read loop for that peer.
async fn register_connection(
  peers: PeerMap,
  disconnecting: DisconnectSet,
  events_tx: mpsc::Sender<NetEvent>,
  conn: Connection,
  peer_id: String,
  emit_connected_event: bool,
) {
  {
    let mut locked = peers.lock().await;
    locked.insert(peer_id.clone(), conn.clone());
  }
  if emit_connected_event {
    let _ = events_tx
      .send(NetEvent::PeerConnected {
        peer_id: peer_id.clone(),
      })
      .await;
  }

  tokio::spawn(run_peer_receive_loop(
    peers,
    disconnecting,
    events_tx,
    conn,
    peer_id,
  ));
}

// Accepts bi-streams from a single connected peer and emits chat events.
async fn run_peer_receive_loop(
  peers: PeerMap,
  disconnecting: DisconnectSet,
  events_tx: mpsc::Sender<NetEvent>,
  conn: Connection,
  peer_id: String,
) {
  loop {
    let (_, mut recv) = match conn.accept_bi().await {
      Ok(streams) => streams,
      Err(err) => {
        log::info!("Peer {peer_id} stream loop ended: {err}");
        break;
      }
    };

    let payload = match recv.read_to_end(MAX_MESSAGE_BYTES).await {
      Ok(data) => data,
      Err(err) => {
        log::warn!("Failed reading incoming message from {peer_id}: {err}");
        continue;
      }
    };

    let message = match String::from_utf8(payload) {
      Ok(message) => message,
      Err(err) => {
        log::warn!("Incoming message from {peer_id} was invalid UTF-8: {err}");
        continue;
      }
    };

    let timestamp_secs = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .map(|d| d.as_secs() as i64)
      .unwrap_or(0);
    
    let _ = events_tx
      .send(NetEvent::PeerMessage {
        peer_id: peer_id.clone(),
        message,
        timestamp_secs,
      })
      .await;
  }

  {
    let mut locked = peers.lock().await;
    locked.remove(&peer_id);
  }

  let intentional_disconnect = {
    let mut disconnecting = disconnecting.lock().await;
    disconnecting.remove(&peer_id)
  };

  if !intentional_disconnect {
    let _ = events_tx.send(NetEvent::PeerDisconnected { peer_id }).await;
  }
}
