use anyhow::Result;
use iroh::endpoint::Connection;
use iroh::{Endpoint, EndpointAddr, PublicKey};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::message::{Message, PeerInfo, Response};

/// In-memory message storage for each peer
pub type MessageStore = Arc<RwLock<HashMap<String, Vec<Message>>>>;

/// Active QUIC connections to peers
pub type ConnectionStore = Arc<RwLock<HashMap<String, Connection>>>;

/// Daemon state
#[derive(Clone)]
pub struct DaemonState {
  /// Iroh endpoint for p2p communication
  pub endpoint: Endpoint,
  /// Message storage: peer_id -> messages
  pub messages: MessageStore,
  /// Connected peers: peer_id -> endpoint address string
  pub peers: Arc<RwLock<HashMap<String, String>>>,
  /// Active QUIC connections: peer_id -> connection
  pub connections: ConnectionStore,
  /// Broadcast channel for incoming messages
  pub msg_broadcast: broadcast::Sender<Message>,
}

impl DaemonState {
  /// Initialize a new daemon with Iroh endpoint
  pub async fn new() -> Result<Self> {
    // Initialize Iroh endpoint with default relay servers
    let alpns = vec![b"p2p-chat".to_vec()];
    let endpoint = Endpoint::builder()
      .alpns(alpns)
      .bind()
      .await?;

    let (msg_broadcast, _) = broadcast::channel(100);

    Ok(DaemonState {
      endpoint,
      messages: Arc::new(RwLock::new(HashMap::new())),
      peers: Arc::new(RwLock::new(HashMap::new())),
      connections: Arc::new(RwLock::new(HashMap::new())),
      msg_broadcast,
    })
  }

  /// Get endpoint ID as string
  pub fn node_id(&self) -> String {
    self.endpoint.id().to_string()
  }

  /// Get addressing information for this endpoint
  pub fn endpoint_addr(&self) -> EndpointAddr {
    self.endpoint.addr()
  }

  /// Connect to a remote peer by endpoint address string
  pub async fn connect_peer(&self, addr_str: &str) -> Result<String> {
    // Parse node ID from address string
    let node_id: PublicKey = addr_str.parse()?;
    let peer_id = node_id.to_string();

    // Build endpoint address with relay discovery
    let addr = EndpointAddr::new(node_id);

    // Establish QUIC connection
    let conn = self.endpoint.connect(addr, b"p2p-chat").await?;
    
    // Store connection
    {
      let mut connections = self.connections.write().await;
      connections.insert(peer_id.clone(), conn);
    }

    // Store peer endpoint
    {
      let mut peers = self.peers.write().await;
      peers.insert(peer_id.clone(), addr_str.to_string());
    }

    // Initialize empty message storage for this peer
    {
      let mut messages = self.messages.write().await;
      messages.entry(peer_id.clone()).or_insert_with(Vec::new);
    }

    Ok(peer_id)
  }

  /// Store an incoming message
  pub async fn store_message(&self, peer_id: String, message: Message) {
    let mut messages = self.messages.write().await;
    messages
      .entry(peer_id)
      .or_insert_with(Vec::new)
      .push(message.clone());
    
    // Broadcast to CLI clients
    let _ = self.msg_broadcast.send(message);
  }

  /// Get all connected peers
  pub async fn list_peers(&self) -> Vec<PeerInfo> {
    let peers = self.peers.read().await;
    let connections = self.connections.read().await;
    peers
      .iter()
      .map(|(id, endpoint)| PeerInfo {
        peer_id: id.clone(),
        endpoint: endpoint.clone(),
        connected: connections.contains_key(id),
      })
      .collect()
  }

  /// Send a message to a peer over QUIC
  pub async fn send_message_to_peer(&self, peer_id: &str, msg: &Message) -> Result<()> {
    let connections = self.connections.read().await;
    let conn = connections
      .get(peer_id)
      .ok_or_else(|| anyhow::anyhow!("Peer not connected"))?;

    // Open a unidirectional stream
    let mut send_stream = conn.open_uni().await?;

    // Serialize message as JSON
    let json = serde_json::to_string(msg)?;
    
    // Send message
    use tokio::io::AsyncWriteExt;
    send_stream.write_all(json.as_bytes()).await?;
    send_stream.finish()?;

    // Store in local history
    self.store_message(peer_id.to_string(), msg.clone()).await;

    Ok(())
  }

  /// Handle an incoming connection from a peer
  pub async fn handle_incoming_connection(&self, conn: Connection) -> Result<()> {
    let peer_id = conn.remote_id().to_string();

    // Store the connection (making it bidirectional)
    {
      let mut connections = self.connections.write().await;
      connections.insert(peer_id.clone(), conn.clone());
    }

    // Store peer info
    {
      let mut peers = self.peers.write().await;
      peers.insert(peer_id.clone(), peer_id.clone());
    }

    // Initialize message storage
    {
      let mut messages = self.messages.write().await;
      messages.entry(peer_id.clone()).or_insert_with(Vec::new);
    }

    // Spawn task to receive messages from this peer
    let state = self.clone();
    let peer_id_clone = peer_id.clone();
    tokio::spawn(async move {
      if let Err(e) = state.receive_messages_from_peer(conn, peer_id_clone).await {
        tracing::error!("Error receiving messages: {}", e);
      }
    });

    Ok(())
  }

  /// Receive messages from a peer connection
  async fn receive_messages_from_peer(&self, conn: Connection, peer_id: String) -> Result<()> {
    loop {
      // Accept incoming unidirectional stream
      let mut recv_stream = conn.accept_uni().await?;

      // Read message data
      let buffer = recv_stream.read_to_end(1024 * 1024).await?; // 1MB limit

      // Parse message
      let msg: Message = serde_json::from_slice(&buffer)?;

      // Store message
      self.store_message(peer_id.clone(), msg).await;
    }
  }

  /// Get status response
  pub async fn get_status(&self) -> Response {
    let addr = self.endpoint_addr();
    let endpoints: Vec<String> = addr
      .ip_addrs()
      .map(|a| a.to_string())
      .collect();

    Response::Status {
      node_id: self.node_id(),
      local_endpoints: endpoints,
      peers: self.list_peers().await,
    }
  }

  /// Get and clear messages for a peer
  pub async fn get_messages(&self, peer_id: &str) -> Vec<Message> {
    let mut messages = self.messages.write().await;
    messages.remove(peer_id).unwrap_or_default()
  }

  /// Shutdown the daemon gracefully
  pub async fn shutdown(&self) -> Result<()> {
    self.endpoint.close().await;
    Ok(())
  }
}
