//! Loggers for daemon and client processes.

mod error;

use error::Result;
pub use error::{Error, ErrorKind};
use crate::paths::{cli_log_file_spec, daemon_log_file_spec};

use flexi_logger::{Cleanup, Criterion, Logger, Naming};

pub fn init_client_logger() -> Result<()> {
  Logger::try_with_str("info")
    .map_err(|err| {
      Error::new(ErrorKind::LoggerInitFailed, err)
    })?
    .log_to_file(cli_log_file_spec())
    .format(flexi_logger::detailed_format)
    .rotate(
      Criterion::AgeOrSize(flexi_logger::Age::Hour, 5_000_000),
      Naming::Timestamps,
      Cleanup::KeepLogFiles(9),
    )
    .append()
    .start()
    .map_err(|err| {
      Error::new(ErrorKind::LoggerInitFailed, err)
    })?;

  Ok(())
}

pub fn init_daemon_logger() -> Result<()> {
  Logger::try_with_str("info")
    .map_err(|err| {
      Error::new(ErrorKind::LoggerInitFailed, err)
    })?
    .log_to_file(daemon_log_file_spec())
    .format(flexi_logger::detailed_format)
    .rotate(
      Criterion::AgeOrSize(flexi_logger::Age::Hour, 5_000_000),
      Naming::Timestamps,
      Cleanup::KeepLogFiles(9),
    )
    .append()
    .start()
    .map_err(|err| {
      Error::new(ErrorKind::LoggerInitFailed, err)
    })?;

  Ok(())
}
