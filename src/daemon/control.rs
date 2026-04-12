use p2p_chat::{
	protocol::{INTERNAL_DAEMON_INIT_FLAG, DAEMON_NAME},
	socket,
	pid
};

use std::{fs, path::PathBuf};
use nix::{
	sys::signal as nix_signal,
};

pub fn create() {
	if let Some(pid) = status() {
		println!("Daemon is already running with PID {}; aborting", pid);
		return;
	} /* TODO: If PID exists, it doesn't necessarily 
	mean the process is precisely our daemon. Implement this check. 
	Stronger long-term design for the future: pair this with a lockfile or Unix socket. */ 

	let exe = std::env::current_exe()
		.expect("failed to get current executable path");

	let mut command = std::process::Command::new(exe);
	command
		.arg(format!("--{}", INTERNAL_DAEMON_INIT_FLAG)) // trigger hidden flag to call real daemon
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null());
	
	/* Detach from controlling terminal using standard new session strategy. std::process's 
	implementation of setsid() is still nightly-only feature, so use nix crate's alternative.*/
	unsafe {
		use std::os::unix::process::CommandExt; // for pre_exec()
		command.pre_exec(|| {
			nix::unistd::setsid().map_err(
				|e: nix::errno::Errno| 
				std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
			).expect("failed to create new session");
			Ok(())
		});
	}

	let child = command.spawn()
		.expect("failed to spawn daemon process");

	println!("Daemon started with pid {}", child.id()); // TODO: Consider instead returning PID from this func
}

pub fn destroy() {
	if let None = status() {
		println!("Daemon is already not running; aborting");
		return;
	}
	
	let pid_fp = pid_file_path();
	let pid = pid::read_from_file(&pid_fp).unwrap();

	match nix_signal::kill(pid, nix_signal::Signal::SIGTERM) {
		Ok(()) => {
			println!("Shutting down the daemon with PID {}\nCheck its status with `daemon status` command", pid);
		},
		Err(nix::errno::Errno::ESRCH) => {
			pid::remove_file(&pid_fp);
			println!("Daemon process with PID {} does not exist. Removed stale PID file", pid);
			return;
		},
		Err(nix::errno::Errno::EPERM) => {
			println!("Permission denied while stopping daemon with PID {}", pid);
			return;
		},
		Err(_) => { // TODO: Handle error brought by Err(_)
			println!("Failed to stop daemon with PID {}", pid);
			return;
		}
	}
	// TODO: Implement waiting for process termination with timeout and forced kill 
	// if it doesn't terminate gracefully.
}

pub fn status() -> Option<i32> {
	let pid_fp = pid_file_path();
	let mut pid: Option<i32> = None;

	if let Some(p) = pid::read_from_file(&pid_fp) {
		if is_process_alive(p) {
			pid = Some(p.into());
		} else {
			pid::remove_file(&pid_fp);
			println!("Found stale daemon PID-file (PID {}) and removed it", p);
		}
	}

	let socket_path = socket_file_path();
	match socket_path.exists() {
		true => {
			if let None = pid {
				socket::remove_file(&socket_path);
				println!("Found stale daemon socket file and removed it");
			}
		},
		false => {
			if let Some(_) = pid { // TODO
				println!("Daemon is running, but socket file is not found; TODO: This is bad so do sth about it");
			}
		}
	}

	pid
}

// ========================================================================
// Helpers
// ========================================================================

fn daemon_state_dir() -> PathBuf {
	if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
		let runtime_path = PathBuf::from(runtime_dir).join("p2pchat");
		if fs::create_dir_all(&runtime_path).is_ok() {
			return runtime_path;
		}
	}

	let home = std::env::var_os("HOME")
		.expect("$HOME environment variable is not set");
	let cache_path = PathBuf::from(home).join(".cache/p2pchat");
	fs::create_dir_all(&cache_path)
		.expect("failed to create daemon state directory");
	
	cache_path
}

pub fn socket_file_path() -> PathBuf {
	daemon_state_dir().join(format!("{DAEMON_NAME}.sock"))
}

pub fn pid_file_path() -> PathBuf {
	daemon_state_dir().join(format!("{DAEMON_NAME}.pid"))
}

fn is_process_alive(pid: nix::unistd::Pid) -> bool {
	match nix_signal::kill(pid, None) {
		Ok(()) => true,
		Err(nix::errno::Errno::EPERM) => true,
		Err(nix::errno::Errno::ESRCH) => false,
		Err(_) => false,
	} /* TODO: This should be taken in account: If sending a null signal fails with the error ESRCH, 
	then we know the process doesn’t exist. If the call fails with the error EPERM 
	(meaning the process exists, but we don’t have permission to send a signal to it) 
	or succeeds (meaning we do have permission to send a signal to the process), 
	then we know that the process exists. */
}
