//! Client module for communicating with the daemon process.

use crate::client::{Error, ErrorKind, Result};
use p2p_chat::{paths, schemas, socket};

use tokio::net::{
  unix::{OwnedReadHalf, OwnedWriteHalf},
  UnixStream as TokioUnixStream,
};

pub(crate) struct Session {
  reader: OwnedReadHalf,
  writer: OwnedWriteHalf,
}

impl Session {
  pub(crate) async fn new() -> Result<Self> {
    let socket_path = paths::daemon_socket();
    let stream = TokioUnixStream::connect(&socket_path)
      .await
      .inspect_err(|err| {
        log::debug!(
          "new() failed to connect to daemon socket at path {}: {err:?}",
          socket_path.display()
        );
      })
      .map_err(|err| Error::other(err))?;

    let (reader, writer) = stream.into_split();
    Ok(Self { reader, writer })
  }

  pub(crate) async fn send_request_to_daemon(
    &mut self,
    request: &schemas::ClientRequest,
  ) -> Result<()> {
    let json = serde_json::to_string(request)
      .inspect_err(|err| {
        log::debug!("send_request_to_daemon() failed to serialize request: {err:?}");
      })
      .map_err(|err| Error::other(err))?;

    socket::write_data(&mut self.writer, &json)
      .await
      .inspect_err(|err| {
        log::debug!("send_request_to_daemon() failed to write data to socket: {err:?}");
      })
      .map_err(|err| {
        if err.kind() == socket::ErrorKind::ConnectionAborted {
          Error::new(ErrorKind::DaemonAbortedConnection, err)
        } else {
          Error::other(err)
        }
      })
  }

  pub(crate) async fn recv_daemon_event(&mut self) -> Result<schemas::DaemonEvent> {
    let json = socket::read_data(&mut self.reader)
      .await
      .inspect_err(|err| {
        log::debug!("recv_daemon_event() failed to read data from socket: {err:?}");
      })
      .map_err(|err| {
        if err.kind() == socket::ErrorKind::ConnectionAborted {
          Error::new(ErrorKind::DaemonAbortedConnection, err)
        } else {
          Error::other(err)
        }
      })?;

    serde_json::from_str::<schemas::DaemonEvent>(&json)
      .inspect_err(|err| {
        log::debug!("recv_daemon_event() failed to deserialize event: {err:?}")
      })
      .map_err(|err| Error::other(err))
  }
}
