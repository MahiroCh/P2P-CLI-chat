//! Daemon transport API for client connections.

use crate::daemon::{Error, ErrorKind, Result};
use p2p_chat::{paths, schemas, socket};

use tokio::net::{
  unix::{OwnedReadHalf, OwnedWriteHalf},
  UnixListener as TokioUnixListener,
};

// == Daemon session management ==

pub(super) struct Session {
  listener: TokioUnixListener,
  reader: Option<OwnedReadHalf>,
  writer: Option<OwnedWriteHalf>,
}

impl Drop for Session {
  fn drop(&mut self) {
    self.reader = None;
    self.writer = None;
  }
}

impl Session {
  pub(super) fn new() -> Result<Self> {
    let socket_path = paths::daemon_socket();
    let listener = socket::create(&socket_path)
      .map_err(|err| Error::other(err))?;

    Ok(Self {
      listener,
      reader: None,
      writer: None,
    })
  }

  pub(super) async fn accept_client(&mut self) -> Result<()> {
    let (stream, _) = self
      .listener
      .accept()
      .await
      .inspect_err(|err| log::debug!("accept_client() failed: {err:?}"))
      .map_err(|err| Error::new(ErrorKind::ClientAcceptFailed, err))?;

    let (reader, writer) = stream.into_split();
    self.reader = Some(reader);
    self.writer = Some(writer);

    Ok(())
  }

  pub(super) async fn recv_client_request(&mut self) -> Result<schemas::ClientRequest> {
    let reader = self
      .reader
      .as_mut()
      .ok_or_else(|| Error::other("no client connected"))?;

    let json = match socket::read_data(reader).await {
      Ok(s) => s,
      Err(err) => {
        if err.kind() == socket::ErrorKind::ConnectionAborted {
          log::debug!(
            "recv_request() failed because client aborted connection: {err:?}"
          );
          return Err(Error::new(ErrorKind::ClientAbortedConnection, err));
        } else {
          log::debug!("recv_request() failed to read data from socket: {err:?}");
          return Err(Error::other(err));
        }
      }
    };

    serde_json::from_str::<schemas::ClientRequest>(&json)
      .inspect_err(|err| {
        log::debug!("recv_request() failed to deserialize request: {err:?}")
      })
      .map_err(|err| Error::other(err))
  }

  pub(super) async fn send_event_to_client(
    &mut self,
    event: &schemas::DaemonEvent,
  ) -> Result<()> {
    let writer = self
      .writer
      .as_mut()
      .ok_or_else(|| Error::other("no client connected"))?;

    let json = serde_json::to_string(event)
      .inspect_err(|err| {
        log::debug!("send_event() failed to serialize event: {err:?}")
      })
      .map_err(|err| Error::other(err))?;

    match socket::write_data(writer, &json).await {
      Ok(()) => Ok(()),
      Err(err) => {
        if err.kind() == socket::ErrorKind::ConnectionAborted {
          log::debug!("send_event() reported client aborted connection: {err:?}");
          Err(Error::new(ErrorKind::ClientAbortedConnection, err))
        } else {
          log::debug!("send_event() failed to write data to socket: {err:?}");
          Err(Error::other(err))
        }
      }
    }
  }
}
