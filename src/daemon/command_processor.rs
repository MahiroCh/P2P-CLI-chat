#[allow(unused)]
use crate::daemon::{Connection, Error, ErrorKind, Result};
use p2p_chat::schemas::ActionCmd;

// NOTE: Temp behavior for testing — echo back received command.
pub(super) async fn process(
  connection: &mut Connection,
  client_id: u32,
  cmd: &ActionCmd,
) -> Result<()> {
  match connection.write_response(&cmd).await {
    Ok(_) => {
      log::info!("Sent response to client (ID {client_id}): {cmd:?}");
    }
    Err(err) if matches!(err.kind(), ErrorKind::ClientAbortedConnection) => {
      log::info!(
        "Client (ID {client_id}) closed connection while daemon was writing \
         response (client process terminated): {err:?}"
      );
      return Err(err);
    }
    Err(err) => {
      log::error!(
        "Failed to write response to client (ID {client_id}) in daemon: {err}"
      );
      return Err(err);
    }
  }

  Ok(())
}