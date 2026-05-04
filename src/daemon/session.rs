//! Daemon transport API for client connections.

use crate::daemon::{Error, ErrorKind, Result};
use p2p_chat::{
  paths,
  schemas::{ActionCmd, DaemonHandshake},
  socket,
};

use std::sync::{
  atomic::{AtomicU32, Ordering},
  Arc,
};
use tokio::net::{UnixListener as TokioUnixListener, UnixStream as TokioUnixStream};

// == Daemon session management ==

pub(super) struct ConnectionsSession {
  listener: TokioUnixListener,
  current_connections: Arc<AtomicU32>,
  pub max_connections: u32,
}

impl ConnectionsSession {
  pub(super) fn new(max_connections: u32) -> Result<Self> {
    let socket_path = paths::daemon_socket();
    let listener = socket::create(&socket_path)
      .inspect_err(|err| {
        log::debug!(
          "socket::create() in daemon::ConnectionsSession::new() failed to create daemon socket: {err:?}"
        );
      })
      .map_err(|err| Error::new(ErrorKind::SocketCreationFailed, err))?;

    Ok(Self {
      listener,
      current_connections: Arc::new(AtomicU32::new(0)),
      max_connections,
    })
  }

  pub(super) async fn accept_connection(&self) -> Result<Connection> {
    let (mut stream, _) = self
      .listener
      .accept()
      .await
      .inspect_err(|err| {
        log::debug!(
          "ConnectionsSession::accept_connection() \
           tokio::net::UnixListener::listen() failed: {err:?}"
        );
      })
      .map_err(|err| Error::new(ErrorKind::ConnectionAcceptFailed, err))?;

    if !try_add_connection(&self.current_connections, self.max_connections) {
      let busy_msg = DaemonHandshake::Busy {
        reason: format!(
          "Maximum concurrent connections ({}) reached",
          self.max_connections
        ),
      };

      send_handshake(&mut stream, &busy_msg).await?;

      log::debug!(
        "ConnectionsSession::accept_connection() rejected new connection and \
         sent handshake about this in due to max connections reached"
      );

      Err(Error::from(ErrorKind::ConnectionAtCapacity))
    } else {
      let ok_msg = DaemonHandshake::Ok;

      send_handshake(&mut stream, &ok_msg).await.map_err(|err| {
        remove_connection(&self.current_connections);
        err
      })?;

      log::debug!(
        "Accepted new connection after handshake in ConnectionsSession::accept_connection()"
      );

      Ok(Connection {
        stream,
        connection_counter: Arc::clone(&self.current_connections),
      })
    }
  }
}

// == Client connection abstraction ==

pub(super) struct Connection {
  stream: TokioUnixStream,
  connection_counter: Arc<AtomicU32>,
}

impl Connection {
  pub(super) async fn read_command(&mut self) -> Result<ActionCmd> {
    let json = match socket::read_data(&mut self.stream).await {
      Ok(s) => s,
      Err(err) => match err.kind() {
        socket::ErrorKind::ConnectionAborted => {
          log::debug!(
            "Connection::read_command(): client aborted connection while reading command: {err:?}"
          );
          return Err(Error::new(ErrorKind::ClientAbortedConnection, err));
        }
        _ => {
          log::debug!(
            "Connection::read_command(): failed to read command from client socket: {err:?}"
          );
          return Err(Error::new(ErrorKind::ReadCommandFailed, err));
        }
      },
    };

    let cmd: ActionCmd = serde_json::from_str(&json)
      .inspect_err(|err| {
        log::debug!(
          "Connection::read_command() failed to deserialize ActionCmd: {err:?}"
        );
      })
      .map_err(|err| Error::new(ErrorKind::SerdeFailed, err))?;

    Ok(cmd)
  }

  pub(super) async fn write_response(&mut self, response: &ActionCmd) -> Result<()> {
    let json = serde_json::to_string(response)
      .inspect_err(|err| {
        log::debug!(
          "Connection::write_response() failed to serialize command: {err:?}"
        );
      })
      .map_err(|err| Error::new(ErrorKind::SerdeFailed, err))?;

    match socket::write_data(&mut self.stream, &json).await {
      Ok(()) => {}
      Err(err) => match err.kind() {
        socket::ErrorKind::ConnectionAborted => {
          log::debug!(
            "Connection::write_response(): client aborted connection while writing response: {err:?}"
          );
          return Err(Error::new(ErrorKind::ClientAbortedConnection, err));
        }
        _ => {
          log::debug!(
            "Connection::write_response(): failed to write response to client socket: {err:?}"
          );
          return Err(Error::new(ErrorKind::WriteResponseFailed, err));
        }
      },
    }

    Ok(())
  }
}

impl Drop for Connection {
  fn drop(&mut self) {
    remove_connection(&self.connection_counter);
    log::debug!("Connection dropped with drop(), decremented connection counter");
  }
}

// == Helpers ==

fn try_add_connection(counter: &AtomicU32, max_value: u32) -> bool {
  loop {
    let current = counter.load(Ordering::Acquire);
    if current >= max_value {
      return false;
    }

    if counter
      .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
      .is_ok()
    {
      return true;
    }
  }
}

fn remove_connection(counter: &AtomicU32) {
  loop {
    let current = counter.load(Ordering::Acquire);
    if current == 0 {
      return;
    }

    if counter
      .compare_exchange(current, current - 1, Ordering::AcqRel, Ordering::Acquire)
      .is_ok()
    {
      return;
    }
  }
}

async fn send_handshake(
  stream: &mut TokioUnixStream,
  msg: &DaemonHandshake,
) -> Result<()> {
  let json = serde_json::to_string(msg)
    .inspect_err(|err| {
      log::debug!("send_handshake() failed to serialize handshake message: {err:?}");
    })
    .map_err(|err| Error::new(ErrorKind::SerdeFailed, err))?;

  match socket::write_data(stream, &json).await {
    Ok(()) => Ok(()),
    Err(err) => match err.kind() {
      socket::ErrorKind::ConnectionAborted => {
        log::debug!(
          "send_handshake(): client aborted connection during handshake: {err:?}"
        );
        Err(Error::new(ErrorKind::ClientAbortedConnection, err))
      }
      _ => {
        log::debug!(
          "send_handshake(): failed to write handshake to client socket: {err:?}"
        );
        Err(Error::new(ErrorKind::ConnectionAcceptFailed, err))
      }
    },
  }
}
