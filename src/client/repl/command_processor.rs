#[allow(unused)]
use crate::client::{Error, ErrorKind, Result, daemon_control::ConnectionSession};
use p2p_chat::schemas::ActionCmd;

// Temp behavior for testing.
pub(super) async fn process(
  daemon_client: &mut ConnectionSession,
  action_cmd: &ActionCmd,
) -> Result<()> {
  log::info!("Sending action command to daemon from REPL");

  match daemon_client._send_cmd_to_daemon(action_cmd).await {
    Ok(()) => {}
    Err(err) if matches!(err.kind(), ErrorKind::DaemonAbortedConnection) => {
      log::debug!(
        "During repl::run() _send_cmd_to_daemon: daemon aborted connection \
         while sending action command: {err:?}"
      );
      return Err(err);
    }
    Err(err) => {
      log::debug!(
        "During repl::run() _send_cmd_to_daemon failed to send action command \
         to daemon: {err:?}"
      );
      return Err(err);
    }
  }

  log::info!("Action command sent to daemon from REPL: {action_cmd:?}");

  let response = match daemon_client._recv_response_from_daemon().await {
    Ok(response) => response,
    Err(err) if matches!(err.kind(), ErrorKind::DaemonAbortedConnection) => {
      log::debug!(
        "During repl::run() _recv_response_from_daemon: daemon aborted connection while receiving response: {err:?}"
      );
      return Err(err);
    }
    Err(err) => {
      log::debug!(
        "During repl::run() _recv_response_from_daemon failed to receive response from daemon: {err:?}"
      );
      return Err(err);
    }
  };

  println!("Response from daemon: {:?}\n", response);
  log::info!("Received response from daemon for action command sent from REPL: {response:?}");

  Ok(())
}