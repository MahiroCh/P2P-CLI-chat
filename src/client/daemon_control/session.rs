//! Client module for communicating with the daemon process.

use crate::client::{Error, ErrorKind, Result};
use p2p_chat::{paths, schemas::*, socket};

use tokio::net::UnixStream as TokioUnixStream;

pub(crate) struct ConnectionSession {
  connection: TokioUnixStream,
}

impl ConnectionSession {
  pub(crate) async fn new() -> Result<Self> {
    let socket_path = paths::daemon_socket();
    let mut connection = TokioUnixStream::connect(&socket_path)
      .await
      .inspect_err(|err| {
        log::debug!(
          "ConnectionSession::new() failed to connect to daemon socket at path {:?}: {err:?}",
          &socket_path
        );
      })
      .map_err(|err| Error::new(ErrorKind::DaemonConnectionFailed, err))?;

    handshake(&mut connection).await.inspect_err(|err| {
      log::debug!(
        "ConnectionSession::new() failed during handshake() with daemon: {err:?}"
      );
    })?;

    Ok(Self { connection })
  }

  // NOTE: Temporary function for testing the socket communication,
  // NOTE: to be replaced with real command processing logic later.
  pub(crate) async fn _send_cmd_to_daemon(&mut self, cmd: &ActionCmd) -> Result<()> {
    let json = serde_json::to_string(cmd)
      .inspect_err(|err| {
        log::debug!(
          "ConnectionSession::_send_cmd_to_daemon(): Failed to serialize action \
           command using serde_json: {err:?}"
        );
      })
      .map_err(|err| Error::new(ErrorKind::SerdeFailed, err))?;

    match socket::write_data(&mut self.connection, &json).await {
      Ok(()) => {}
      Err(err) => match err.kind() {
        socket::ErrorKind::ConnectionAborted => {
          log::debug!(
            "ConnectionSession::_send_cmd_to_daemon(): daemon closed connection (abort): {err:?}"
          );
          return Err(Error::new(ErrorKind::DaemonAbortedConnection, err));
        }
        _ => {
          log::debug!(
            "ConnectionSession::_send_cmd_to_daemon(): failed to write command to daemon socket: {err:?}"
          );
          return Err(Error::new(ErrorKind::WriteCommandFailed, err));
        }
      },
    }

    Ok(())
  }

  // NOTE: Temporary function for testing the socket communication,
  // NOTE: to be replaced with real command processing logic later.
  pub(crate) async fn _recv_response_from_daemon(&mut self) -> Result<ActionCmd> {
    let json = match socket::read_data(&mut self.connection).await {
      Ok(s) => s,
      Err(err) => match err.kind() {
        socket::ErrorKind::ConnectionAborted => {
          log::debug!(
            "ConnectionSession::_recv_response_from_daemon(): daemon aborted connection while reading response: {err:?}"
          );
          return Err(Error::new(ErrorKind::DaemonAbortedConnection, err));
        }
        _ => {
          log::debug!(
            "ConnectionSession::_recv_response_from_daemon(): failed to read response from daemon socket: {err:?}"
          );
          return Err(Error::new(ErrorKind::ReadResponseFailed, err));
        }
      },
    };

    let cmd: ActionCmd = serde_json::from_str(&json)
      .inspect_err(|err| {
        log::debug!(
          "ConnectionSession::_recv_response_from_daemon(): Failed to deserialize \
           command with serde_json::from_str() from daemon response in 
           _recv_response_from_daemon: {err:?}"
        );
      })
      .map_err(|err| Error::new(ErrorKind::SerdeFailed, err))?;

    Ok(cmd)
  }
}

// == Helpers ==

async fn handshake(connection: &mut TokioUnixStream) -> Result<()> {
  let handshake_str = match socket::read_data(connection).await {
    Ok(s) => s,
    Err(err) => match err.kind() {
      socket::ErrorKind::ConnectionAborted => {
        log::debug!("ConnectionSession::handshake(): daemon aborted connection during handshake: {err:?}");
        return Err(Error::new(ErrorKind::DaemonAbortedConnection, err));
      }
      _ => {
        log::debug!("ConnectionSession::handshake(): failed to read handshake from daemon socket: {err:?}");
        return Err(Error::new(ErrorKind::DaemonConnectionFailed, err));
      }
    },
  };

  let handshake: DaemonHandshake = serde_json::from_str(&handshake_str)
    .inspect_err(|err| {
      log::debug!(
        "ConnectionSession::handshake() failed to parse daemon handshake JSON with \
         serde_json::from_str(): {err:?}"
      );
    })
    .map_err(|err| Error::new(ErrorKind::DaemonConnectionFailed, err))?;

  match handshake {
    DaemonHandshake::Ok => Ok(()),
    DaemonHandshake::Busy { reason } => {
      Err(Error::new(ErrorKind::DaemonRefusedConnection, reason))
    }
  }
}
