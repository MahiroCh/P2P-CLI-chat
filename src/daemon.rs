use iroh::endpoint::Connection;
use iroh::{Endpoint, EndpointAddr, PublicKey};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::Message;

/// Active QUIC connections to peers
pub type ConnectionStore = Arc<RwLock<HashMap<String, Connection>>>;

/// Daemon state
#[derive(Clone)]
pub struct DaemonState {
  pub endpoint: Endpoint,
  pub connections: ConnectionStore,
}

impl DaemonState {
  pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
    let alpns = vec![b"p2p-chat".to_vec()];
    let endpoint = Endpoint::builder()
      .alpns(alpns)
      .bind()
      .await?;

    Ok(DaemonState {
      endpoint,
      connections: Arc::new(RwLock::new(HashMap::new())),
    })
  }

  pub fn node_id(&self) -> String {
    self.endpoint.id().to_string()
  }

  pub fn endpoint_addr(&self) -> EndpointAddr {
    self.endpoint.addr()
  }

  pub async fn connect_peer(&self, addr_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let node_id: PublicKey = addr_str.parse()?;
    let peer_id = node_id.to_string();
    let addr = EndpointAddr::new(node_id);
    let conn = self.endpoint.connect(addr, b"p2p-chat").await?;
    
    let mut connections = self.connections.write().await;
    connections.insert(peer_id.clone(), conn);

    Ok(peer_id)
  }

  pub async fn send_message_to_peer(&self, peer_id: &str, msg: &Message) -> Result<(), Box<dyn std::error::Error>> {
    let connections = self.connections.read().await;
    let conn = connections
      .get(peer_id)
      .ok_or("Peer not connected")?;

    let mut send_stream = conn.open_uni().await?;
    let json = serde_json::to_string(msg)?;
    
    use tokio::io::AsyncWriteExt;
    send_stream.write_all(json.as_bytes()).await?;
    send_stream.finish()?;

    Ok(())
  }

  pub async fn handle_incoming_connection(&self, conn: Connection) -> Result<(), Box<dyn std::error::Error>> {
    let peer_id = conn.remote_id().to_string();

    {
      let mut connections = self.connections.write().await;
      connections.insert(peer_id.clone(), conn.clone());
    }

    let state = self.clone();
    let peer_id_clone = peer_id.clone();
    tokio::spawn(async move {
      let _ = state.receive_messages_from_peer(conn, peer_id_clone).await;
    });

    Ok(())
  }

  async fn receive_messages_from_peer(&self, conn: Connection, _peer_id: String) -> Result<(), Box<dyn std::error::Error>> {
    loop {
      let mut recv_stream = conn.accept_uni().await?;
      let buffer = recv_stream.read_to_end(1024 * 1024).await?;
      let msg: Message = serde_json::from_slice(&buffer)?;
      
      println!("[{}] {}", msg.sender, msg.content);
    }
  }

  pub async fn shutdown(&self) {
    self.endpoint.close().await;
  }
}
