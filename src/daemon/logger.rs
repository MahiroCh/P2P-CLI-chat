//! Loggers for daemon process.

use crate::daemon::{Error, Result};
use p2p_chat::paths::{daemon_log_file_spec, daemon_iroh_log_file_spec};

use flexi_logger::{
  writers::{FileLogWriter, LogWriter},
  DeferredNow, FileSpec, FlexiLoggerError, Logger,
};

pub fn init(level: String) -> Result<()> {
  // Use the package name provided by Cargo at compile time. Cargo package
  // names may contain hyphens while Rust crate targets use underscores,
  // so convert hyphens to underscores to match log targets.
  let crate_name = env!("CARGO_PKG_NAME").replace('-', "_");
  let spec = format!("info,{}={}", crate_name, level);

  let split_writer = SplitLogWriter::new(
    daemon_log_file_spec(),
    daemon_iroh_log_file_spec(),
  )
  .map_err(Error::other)?;

  Logger::try_with_str(spec)
    .map_err(|err| Error::other(err))?
    .log_to_writer(Box::new(split_writer))
    .format(flexi_logger::detailed_format)
    .start()
    .map_err(|err| Error::other(err))?;

  Ok(())
}

// == Custom LogWriter for split-logging between daemon and Iroh ==

// Iroh logs was interleaving with daemon logs in the same file, flooding it 
// with unintelligible litter and making it difficult to find relevant info about
// the daemon itself. To solve this, I rerouted Iroh logs to a separate file. 

pub(crate) struct SplitLogWriter {
  daemon_writer: FileLogWriter,
  iroh_writer: FileLogWriter,
}

impl SplitLogWriter {
  pub(crate) fn new(
    daemon_file_spec: FileSpec,
    iroh_file_spec: FileSpec,
  ) -> std::result::Result<Self, FlexiLoggerError> {
    let daemon_writer = FileLogWriter::builder(daemon_file_spec)
      .rotate(
        flexi_logger::Criterion::AgeOrSize(flexi_logger::Age::Hour, 5_000_000),
        flexi_logger::Naming::Timestamps,
        flexi_logger::Cleanup::KeepLogFiles(9),
      )
      .append()
      .try_build()?;

    let iroh_writer = FileLogWriter::builder(iroh_file_spec)
      .rotate(
        flexi_logger::Criterion::AgeOrSize(flexi_logger::Age::Hour, 5_000_000),
        flexi_logger::Naming::Timestamps,
        flexi_logger::Cleanup::KeepLogFiles(9),
      )
      .append()
      .try_build()?;

    Ok(Self {
      daemon_writer,
      iroh_writer,
    })
  }
}

use log::Record;
impl LogWriter for SplitLogWriter {
  fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
    if is_iroh_record(record) {
      self.iroh_writer.write(now, record)
    } else {
      self.daemon_writer.write(now, record)
    }
  }

  fn flush(&self) -> std::io::Result<()> {
    self.daemon_writer.flush()?;
    self.iroh_writer.flush()?;
    Ok(())
  }

  fn max_log_level(&self) -> log::LevelFilter {
    log::LevelFilter::Debug
  }

  fn reopen_output(&self) -> std::result::Result<(), FlexiLoggerError> {
    self.daemon_writer.reopen_output()?;
    self.iroh_writer.reopen_output()?;
    Ok(())
  }

  fn rotate(&self) -> std::result::Result<(), FlexiLoggerError> {
    self.daemon_writer.rotate()?;
    self.iroh_writer.rotate()?;
    Ok(())
  }

  fn shutdown(&self) {
    self.daemon_writer.shutdown();
    self.iroh_writer.shutdown();
  }
}

// == Helpers for routing logs between daemon and Iroh files ==

fn is_iroh_record(record: &Record) -> bool {
  is_iroh_target(record.target())
    || record.module_path().map(is_iroh_target).unwrap_or(false)
}

fn is_iroh_target(target: &str) -> bool {
  [
    "iroh",
    "iroh_base",
    "iroh_dns",
    "iroh_relay",
    "n0",
    "n0_future",
    "n0_watcher",
    "noq",
    "noq_udp",
    "portmapper",
    "igd_next",
    "netwatch",
    "netlink_packet_route",
    "quinn",
    "hickory",
    "hickory_proto",
    "hickory_resolver",
    "reqwest",
    "rustls",
    "rustls_webpki",
    "tokio_rustls",
  ]
  .iter()
  .any(|prefix| target == *prefix || target.starts_with(&format!("{prefix}::")))
}
