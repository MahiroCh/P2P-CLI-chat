//! REPL mode for p2p chat.

use crate::client::{
  daemon_session::Session as DaemonSession, Error, ErrorKind, Result,
};
use p2p_chat::{
  cli_interface::InteractiveCommand,
  schemas::{ClientRequest, DaemonEvent},
};

use rustyline_async::{Readline, ReadlineError, ReadlineEvent, SharedWriter};
use std::io::Write;
use tokio::sync::mpsc;

// == REPL logic ==

#[tokio::main]
pub(super) async fn run() -> Result<()> {
  let (mut repl_engine, mut repl_printer, mut daemon_session) =
    init_repl_components()
      .await
      .inspect(|_| log::info!("REPL initialized, client connected to daemon"))
      .inspect_err(|err| log::error!("REPL failed to initialize: {err}"))
      .map_err(|err| Error::new(ErrorKind::ReplInitFailed, err))?;

  let (repl_print_tx, mut repl_print_rx) = mpsc::unbounded_channel::<String>();

  loop {
    tokio::select! {
      out = repl_engine.readline() => {
        match logic(&mut repl_engine, out, &mut daemon_session).await {
          Ok(LogicOut::Continue) => {}
          Ok(LogicOut::Quit { message }) => {
            let _ = repl_print_tx.send(message);
            // Drain any remaining messages to print before quitting.
            while let Ok(message) = repl_print_rx.try_recv() {
              print_repl_message(&mut repl_printer, &message)?;
            }
            let _ = repl_engine.flush();
            let _ = daemon_session.send_request_to_daemon(&ClientRequest::Bye).await;
            break;
          }
          Ok(LogicOut::Print(message)) => {
            let _ = repl_print_tx.send(message);
          }
          Err(err) => {
            if err.kind() == ErrorKind::DaemonAbortedConnection {
              return Err(err);
            }
            return Err(Error::other(err));
          }
        }
      }
      event = daemon_session.recv_daemon_event() => {
        match event {
          Ok(event) => {
            let _ = repl_print_tx.send(render_daemon_event(event));
          }
          Err(err) => {
            if err.kind() == ErrorKind::DaemonAbortedConnection {
              return Err(err);
            }
            return Err(Error::other(err));
          }
        }
      }
      message = repl_print_rx.recv() => {
        if let Some(message) = message {
          print_repl_message(&mut repl_printer, &message)?;
        }
      }
    }
  }

  Ok(())
}

enum LogicOut {
  Print(String),
  Continue,
  Quit { message: String },
}

async fn logic(
  repl_engine: &mut Readline,
  readlined: std::result::Result<ReadlineEvent, ReadlineError>,
  daemon_session: &mut DaemonSession,
) -> Result<LogicOut> {
  match readlined {
    Ok(ReadlineEvent::Line(raw)) => {
      let raw_input = raw.trim();
      if raw_input.is_empty() {
        return Ok(LogicOut::Continue);
      }
      repl_engine.add_history_entry(raw_input.to_owned());

      match parse_raw_input(raw_input) {
        Ok(InteractiveCommand::Quit) => {
          log::info!("User issued quit command in REPL mode, quitting...");
          log::debug!(
            "logic()'s parse_raw_input() match arm indicated that \
             user issued quit command in REPL mode, quitting..."
          );
          return Ok(LogicOut::Quit {
            message: "Quitting interactive mode...".to_owned(),
          });
        }
        Ok(action) => {
          log::info!("User issued request to daemon in REPL mode: {action:?}");
          log::debug!(
            "logic()'s parse_raw_input() match arm indicated that \
             user issued request to daemon in REPL mode: {action:?}"
          );
          daemon_session
            .send_request_to_daemon(&ClientRequest::from(action))
            .await
            .inspect_err(|err| {
              log::debug!(
                "logic()'s daemon_session.send_request() failed to \
                 send request to daemon: {err:?}"
              );
            })?;
          return Ok(LogicOut::Continue);
        }
        Err(err) => {
          return Ok(LogicOut::Print(err.to_string()));
        }
      }
    }
    Ok(ReadlineEvent::Interrupted) => {
      log::info!(
        "User issued interrupt (Ctrl+C) in REPL mode. Continuing \
         because REPL is designed to ignore interrupts and quit on special command"
      );
      return Ok(LogicOut::Print(
        "\nUse `quit` to exit interactive mode".to_owned(),
      ));
    }
    Ok(ReadlineEvent::Eof) => {
      log::info!("User issued EOF (Ctrl+D) in REPL mode, quitting...");
      log::debug!(
        "logic()'s match arm for Ok(ReadlineEvent::Eof) indicates that \
         user issued EOF (Ctrl+D) in REPL mode, quitting..."
      );
      return Ok(LogicOut::Quit {
        message: "EOF received, quitting interactive mode...".to_owned(),
      });
    }
    Err(err) => {
      return Err(Error::other(err));
    }
  }
}

// == Helpers ==

fn print_repl_message(repl_printer: &mut SharedWriter, message: &str) -> Result<()> {
  writeln!(repl_printer, "{message}")
    .inspect_err(|err| {
      log::debug!("print_repl_message() failed to write to terminal: {err:?}");
    })
    .map_err(Error::other)
}

fn render_daemon_event(event: DaemonEvent) -> String {
  match event {
    DaemonEvent::MyId { endpoint_id } => format_myid_output(&endpoint_id),
    DaemonEvent::PeerList { peers } => {
      if peers.is_empty() {
        "no peers connected".to_owned()
      } else {
        let mut lines = String::from("connected peers:");
        for p in peers {
          lines.push_str("\n  - ");
          lines.push_str(&p);
        }
        lines
      }
    }
    DaemonEvent::Ok { info } => {
      format!("ok: {info}")
    }
    DaemonEvent::Error { message } => {
      format!("error: {message}")
    }
    DaemonEvent::PeerConnected { peer_id } => {
      format!("*** peer connected: {peer_id}")
    }
    DaemonEvent::PeerDisconnected { peer_id } => {
      format!("*** peer disconnected: {peer_id}")
    }
    DaemonEvent::PeerMessage { peer_id, message, timestamp_secs } => {
      let timestamp = format_timestamp(timestamp_secs);
      format!("[{timestamp}] <{peer_id}> {message}")
    }
    _ => unreachable!("[non-exhaustive] unhandled daemon event: {event:?}"),
  }
}

fn format_timestamp(secs: i64) -> String {
  use chrono::{TimeZone, Utc, Local};
  
  match Utc.timestamp_opt(secs, 0) {
    chrono::LocalResult::Single(dt) => {
      dt.with_timezone(&Local).format("%H:%M:%S").to_string()
    },
    chrono::LocalResult::Ambiguous(dt, _) => {
      dt.with_timezone(&Local).format("%H:%M:%S").to_string()
    },
    chrono::LocalResult::None => "<invalid-time>".to_owned(),
  }
}

fn format_myid_output(ticket: &str) -> String {
  let endpoint_id = serde_json::from_str::<iroh::EndpointAddr>(ticket)
    .map(|addr| addr.id.to_string())
    .unwrap_or_else(|_| "<failed-to-parse-endpoint-id>".to_owned());

  format!(
    "my endpoint id: {endpoint_id}\n\
     my ticket: {ticket}\n\
     tip: connect using either endpoint id or full ticket"
  )
}

fn parse_raw_input(raw: &str) -> Result<InteractiveCommand> {
  let argv = match shlex::split(raw) {
    Some(v) => v,
    None => {
      log::debug!(
        "parse_raw_input() failed to parse input {raw} into argv using shlex"
      );
      use clap::CommandFactory;
      let formatted_err = InteractiveCommand::command()
        .error(
          clap::error::ErrorKind::InvalidValue,
          "command of invalid format",
        )
        .render()
        .ansi()
        .to_string();
      return Err(Error::new(ErrorKind::ShlexFailed, formatted_err));
    }
  };
  use clap::Parser;
  let cmd = match InteractiveCommand::try_parse_from(&argv) {
    Ok(cmd) => cmd,
    Err(err) => {
      log::debug!(
        "parse_raw_input() failed to parse {argv:?} into command using clap: {err}"
      );
      let formatted_err = err.render().ansi().to_string();
      return Err(Error::new(ErrorKind::ClapFailed, formatted_err));
    }
  };

  Ok(cmd)
}

async fn init_repl_components() -> Result<(Readline, SharedWriter, DaemonSession)> {
  let (rl, wl) = Readline::new("> ".to_owned())
    .inspect_err(|err| {
      log::debug!(
        "init_repl_components() failed to initialize repl engine: {err:?}"
      );
    })
    .map_err(|err| {
      Error::other(format!("failed to initialize repl engine: {err}"))
    })?;

  let session = DaemonSession::new()
    .await
    .inspect_err(|err| {
      log::debug!(
        "init_repl_components() failed to create session with daemon: {err:?}"
      );
    })
    .map_err(|err| {
      Error::other(format!("failed to create session with daemon: {err}"))
    })?;

  Ok((rl, wl, session))
}
