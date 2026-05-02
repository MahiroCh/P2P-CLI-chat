//! Paths to various files used by the application.
//!
//! # Socket and pid:
//! - $XDG_RUNTIME_DIR/p2pchat/daemon.sock
//! - $XDG_RUNTIME_DIR/p2pchat/daemon.pid
//! - fallback: ~/.cache/p2pchat/...
//!
//! # Logs:
//! - $XDG_STATE_HOME/p2pchat/daemon.log
//! - $XDG_STATE_HOME/p2pchat/cli.log
//! - fallback: ~/.local/state/p2pchat/...
//!
//! # Durable app data:
//! - $XDG_DATA_HOME/p2pchat/...
//! - fallback: ~/.local/share/p2pchat/...

use flexi_logger::FileSpec;
use std::{env, path::PathBuf};

const APP: &str = "p2pchat";
const DAEMON_FILENAMES_BASE: &str = "daemon";
const CLI_FILENAMES_BASE: &str = "cli";

pub fn daemon_socket() -> PathBuf {
  runtime_dir().join(format!("{DAEMON_FILENAMES_BASE}.sock"))
}

pub fn daemon_pidfile() -> PathBuf {
  runtime_dir().join(format!("{DAEMON_FILENAMES_BASE}.pid"))
}

pub fn daemon_log_file_spec() -> FileSpec {
  log_file_spec(DAEMON_FILENAMES_BASE)
}

pub fn cli_log_file_spec() -> FileSpec {
  log_file_spec(CLI_FILENAMES_BASE)
}

// For flexi_logger crate.
fn log_file_spec(basename: &str) -> FileSpec {
  FileSpec::default()
    .directory(state_dir())
    .basename(basename)
    .suffix("log")
    .suppress_timestamp()
}

fn runtime_dir() -> PathBuf {
  dir_from_env_or_home("XDG_RUNTIME_DIR", ".cache").join(APP)
}

fn state_dir() -> PathBuf {
  dir_from_env_or_home("XDG_STATE_HOME", ".local/state").join(APP)
}

// Not used in the current implementation.
pub fn data_dir() -> PathBuf {
  dir_from_env_or_home("XDG_DATA_HOME", ".local/share").join(APP)
}

fn dir_from_env_or_home(var: &str, fallback_under_home: &str) -> PathBuf {
  env::var_os(var)
    .map(PathBuf::from)
    .filter(|p| !p.as_os_str().is_empty() && p.is_absolute())
    .unwrap_or_else(|| home_dir().join(fallback_under_home))
}

fn home_dir() -> PathBuf {
  // TODO: Consider error handling here.
  env::var_os("HOME")
    .map(PathBuf::from)
    .expect("failed to determine home directory")
}
