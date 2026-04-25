//! PID file management.

use nix::unistd::Pid;
use std::{
  fs,
  path::PathBuf,
};

pub fn this_proc_pid() -> Pid {
  Pid::this()
}

pub fn create_file(path: &PathBuf) {
  remove_file(path);

  let parent = path.parent()
    .expect("pid file path must have a parent directory (at least /run or ~/.cache)");
  fs::create_dir_all(parent)
    .expect("failed to create pid file parent directory");

  let _ = fs::File::create(path)
    .expect("failed to create pid file");
}

pub fn write_to_file(path: &PathBuf, pid: Pid) {
  fs::write(path, pid.to_string())
    .expect("failed to write to pid file");
}

pub fn read_from_file(path: &PathBuf) -> Option<Pid> {
  let content = match fs::read_to_string(path) {
    Ok(value) => value,
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
    Err(err) => panic!("failed to read pid file: {err}"),
  };

  let raw_pid: i32 = content.trim().parse()
    .expect("failed to parse PID as i32");

  Some(Pid::from_raw(raw_pid))
}

pub fn remove_file(path: &PathBuf) {
  match fs::remove_file(path) {
    Ok(()) => {}
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
    Err(err) => panic!("failed to remove pid file: {err}"),
  }

  if let Some(parent) = path.parent() {
    match fs::remove_dir(parent) {
      Ok(()) => {},
      Err(err) 
        if err.kind() == std::io::ErrorKind::NotFound
        || err.kind() == std::io::ErrorKind::DirectoryNotEmpty => {},
      Err(_) => {
        todo!(
          "failed to remove pid file parent directory\n\
           Something unexpected happened, and if it is permission issue, \
           consider implementing some behavior for this case."
        );
      }
    }
  }
}
