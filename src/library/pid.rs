use std::{fs, path::PathBuf};
use nix::unistd::Pid;

pub fn this_proc_pid() -> Pid {
  Pid::from_raw(std::process::id() as i32)
}

pub fn remove_file(path: &PathBuf) {
	match fs::remove_file(path) { // Remove the PID file
		Ok(()) => {},
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => {},
		Err(err) => panic!("failed to remove PID file: {}", err)
	}
	
	if let Some(parent) = path.parent() { // Try to remove the parent directory (p2pchat/)
		match fs::remove_dir(parent) {
			Ok(()) => {},
			Err(err) if err.kind() == std::io::ErrorKind::NotFound => {},
			Err(_) => {
				// TODO: Maybe do sth with the fact that:
				// directory might not be empty or have other permission issues.
				// For now, silently ignore since the PID file is already removed.
			}
		}
	}
}

pub fn create_file(path: &PathBuf) {
	remove_file(path);

	let parent = path.parent()
		.expect("pid file path must have a parent directory (at least /run or ~/.cache)");
	fs::create_dir_all(parent)
		.expect("failed to create PID file parent directory");

	fs::File::create(path)
		.expect("failed to create PID file");
}

pub fn write_to_file(path: &PathBuf, pid: Pid) {
	fs::write(path, pid.to_string())
		.expect("failed to write PID file");
}

pub fn read_from_file(path: &PathBuf) -> Option<Pid> {
	let file_content = match fs::read_to_string(&path) {
		Ok(c) => c,
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
		Err(err) => panic!("failed to read PID file: {}", err),
	};
	let pid = file_content.trim().parse::<u32>()
		.expect("failed to parse PID as u32");
	
	Some(Pid::from_raw(pid as i32))
}