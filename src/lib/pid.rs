//! PID file management.

mod error;

use error::Result;
pub use error::{Error, ErrorKind};

use nix::unistd::Pid;
use std::{fs, path::Path};

// == PID file management ==

pub fn this_proc_pid() -> Pid {
  Pid::this()
}

pub fn create(path: &Path, pid: &Pid) -> Result<()> {
  match cleanup(path) {
    Ok(()) => {}
    Err(err) if matches!(err.kind(), ErrorKind::RemoveParentDir) => {
      // It's bad because this error is caused not because dir is not empty or
      // not found (see cleanup() implementation) but because of some other I/O,
      // but we can still try to continue and write to this file, and if it fails,
      // we'll report that error instead.
    }
    Err(err) => return Err(err),
  }

  let parent = path
    .parent()
    .ok_or_else(|| Error::from(ErrorKind::ParentDirInvalid))?;

  fs::create_dir_all(parent)
    .map_err(|err| Error::new(ErrorKind::CreateParentDir, err))?;
  fs::File::create(path).map_err(|err| Error::new(ErrorKind::CreatePidFile, err))?;
  fs::write(path, pid.to_string())
    .map_err(|err| Error::new(ErrorKind::WriteToPidFile, err))?;

  Ok(())
}

pub fn read(path: &Path) -> Result<Pid> {
  let content = fs::read_to_string(path).map_err(|err| {
    if err.kind() == std::io::ErrorKind::NotFound {
      Error::from(ErrorKind::PidFileNotFound)
    } else {
      Error::new(ErrorKind::ReadFromPidFile, err)
    }
  })?;

  let raw_pid: i32 = content
    .trim()
    .parse()
    .map_err(|err| Error::new(ErrorKind::InvalidPidFileContent, err))?;

  Ok(Pid::from_raw(raw_pid))
}

pub fn cleanup(path: &Path) -> Result<()> {
  match fs::remove_file(path) {
    Ok(()) => {}
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
    Err(err) => return Err(Error::new(ErrorKind::RemovePidFile, err)),
  }

  if let Some(parent) = path.parent() {
    match fs::remove_dir(parent) {
      Ok(()) => {}
      Err(err)
        if matches!(
          err.kind(),
          std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
        ) => {}
      Err(err) => return Err(Error::new(ErrorKind::RemoveParentDir, err)),
    }
  }

  Ok(())
}
