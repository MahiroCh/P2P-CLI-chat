use std::{env, path, fs};
use nix::unistd::Pid;

const DAEMON_NAME: &str = "p2pchat";

pub fn this_proc_id() -> Pid {
  Pid::from_raw(std::process::id() as i32)
}

pub fn pid_file_path() -> path::PathBuf {
	if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
		return path::PathBuf::from(runtime_dir).join(format!("p2pchat/{DAEMON_NAME}.pid"));
	}

	let home = env::var_os("HOME")
		.expect("$HOME environment variable is not set");
	path::PathBuf::from(home).join(format!(".local/share/p2pchat/{DAEMON_NAME}.pid"))
}

pub fn write_pid_file(pid: Pid) {
	let path = pid_file_path();

	let parent = path.parent()
		.expect("PID file path must have a parent directory (at least /run or ~/.local/...)");
	fs::create_dir_all(parent)
		.expect("failed to create PID file parent directory");

	fs::write(path, pid.to_string())
		.expect("failed to write PID file");
}

pub fn remove_pid_file() {
	let path = pid_file_path();
	
	match fs::remove_file(&path) { // Remove the PID file
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

pub fn read_pid_file() -> Option<Pid> {
	let path = pid_file_path();

	let file_content = match fs::read_to_string(&path) {
		Ok(c) => c,
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
		Err(err) => panic!("failed to read PID file: {}", err),
	};
	let pid = file_content.trim().parse::<u32>()
		.expect("failed to parse PID as u32");
	
	Some(Pid::from_raw(pid as i32))
}
