//! Loggers for client processes.

use crate::client::{Error, Result};
use p2p_chat::paths::cli_log_file_spec;

use flexi_logger::{Cleanup, Criterion, Logger, Naming};

pub fn init(level: String) -> Result<()> {
  Logger::try_with_str(level)
    .map_err(|err| Error::other(err))?
    .log_to_file(cli_log_file_spec())
    .format(flexi_logger::detailed_format)
    .rotate(
      Criterion::AgeOrSize(flexi_logger::Age::Hour, 5_000_000),
      Naming::Timestamps,
      Cleanup::KeepLogFiles(9),
    )
    .append()
    .start()
    .map_err(|err| Error::other(err))?;

  Ok(())
}
