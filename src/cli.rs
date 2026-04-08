// TODO: Implement normal error handling

mod cmdline;

use rustyline::{Cmd, error::ReadlineError as RstlnReadlineErr};

pub use cmdline::Cmdline; // for main.rs
use cmdline::*;
use crate::daemon;

pub fn process_cmdline(cmd: Cmdline) { // TODO: Should return Result<...>
  match cmd.command { // TODO: Handle the exit code and errors more gracefully
    Command::Interactive => { 
      if daemon::daemon_status() {
        let _ = interactive_cli();
      } else {
        println!("Daemon is not running. Please start the daemon first with `daemon start` command.");
        // TODO: Return some error
      }
    },

    Command::PeerCommand(peer_cmd) => {  }, // TODO: Implement

    Command::Daemon { command } => {
      match command {
        DaemonCmd::Start { initialize } => {
          if initialize { daemon::daemon(); } 
          else { daemon::start_daemon(); }
        },
        
        DaemonCmd::Stop => { daemon::stop_daemon(); },

        DaemonCmd::Status => { daemon::daemon_status(); }
      }
    }
  }
}

fn interactive_cli() { // Should return Result<...>
  let mut rl = rustyline::DefaultEditor::new().unwrap();

  loop {
    let readline = rl.readline("> ");
    match readline {
      Ok(cmdline) => {
        use clap::Parser;
        let cmdline_args = shlex::split(&cmdline).expect("shlex parsing failed in interactive mode");
        let intrct_cli = match InteractiveCmdline::try_parse_from(cmdline_args) {
          Ok(parsed) => parsed,
          Err(e) => {
            let _ = e.print(); // TODO: Handle errors
            continue;
          },
        };

        match intrct_cli.command {
          InteractiveCommand::Quit => {
            println!("Exited interactive mode");
            // TODO: Return success
          },
          InteractiveCommand::PeerCommand(peer_cmd) => { } // TODO: Implement
        }
      },
      Err(RstlnReadlineErr::Interrupted) => {
        println!("\nUse 'quit' to exit");
        continue;
      },
      Err(err) => {
        eprintln!("Error: {:?}\nDying", err); // TODO: Debug representation of an error may not be user-friendly, consider implementing Display for better error messages
        // TODO: Return failure
      }
    }
  }
}