//! Client module for communicating with the daemon process.

use crate::client::{Error, ErrorKind, Result};
use p2p_chat::{paths, socket, cli_schema::*};

use tokio::net::UnixStream as TokioUnixStream;

pub struct Session {
  connection: TokioUnixStream,
}

impl Session {
  pub async fn connect() -> Result<Self> {
    let socket_path = paths::daemon_socket();
    let connection = TokioUnixStream::connect(&socket_path).await
      .map_err(|err| {
        log::debug!("Failed to connect to daemon socket at path {:?}: {err:?}", &socket_path);
        Error::new(ErrorKind::DaemonConnectionFailed, err)
      })?;

    Ok ( 
      Self {
        connection,
      }
    )
  }

  // NOTE: Temporary function for testing the socket communication,
  // NOTE: to be replaced with real command processing logic later.
  pub async fn _test_handle_peer_command(&mut self, cmd: &PeerCmd) -> Result<String> {
    let serded_cmd = serde_json::to_string(cmd).map_err(|err| {
      log::debug!("Failed to serialize peer command using serde_json in _test_handle_peer_command: {err:?}");
      Error::new(ErrorKind::SerdeFailed, err)
    })?;

    socket::write_data(&mut self.connection, &serded_cmd).await
      .map_err(|err| {
        log::debug!("Failed to write peer command to daemon socket in _test_handle_peer_command: {err:?}");
        Error::new(ErrorKind::WriteCommandFailed, err)
      })?;

    let response = socket::read_data(&mut self.connection).await
      .map_err(|err| {
        log::debug!("Failed to read response from daemon socket in _test_handle_peer_command: {err:?}");
        Error::new(ErrorKind::ReadResponseFailed, err)
      })?;

    Ok(response)
  }
}
