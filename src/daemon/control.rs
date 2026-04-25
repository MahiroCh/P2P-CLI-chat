//! Daemon control module: responsible for managing the lifecycle of the daemon 
//! process, including starting, stopping, and checking its status.

// NOTE: Consider implementing some way for daemon to kill itself if it 
// NOTE: idles without CLI connection for too long to avoid dangling process.
// NOTE: (which is not really bad as I know, but still...)

use p2p_chat::{
	cli_schema::{INTERNAL_DAEMON_INIT_FLAG, DAEMON_NAME},
	socket,
	pid
};

use std::{
	fs, 
	path::PathBuf
};
use nix::{
	sys::signal as nix_signal,
};

// Driver code.

pub fn create() {
	// TODO: If PID exists, it doesn't necessarily mean the process is 
	// TODO: our p2p-chat daemon. Some other proces might have taken the PID.
	// TODO: Implement this check.
	// NOTE: Stronger design for the future: pair this with a lockfile or Unix socket.
	if let Some(pid) = status() {
		println!("Daemon is already running with PID {}; aborting", pid);
		return;
	} 

	// Get current binary path to spawn the same binary with a hidden flag 
	// that triggers the real daemon initialization code.
	let exe = std::env::current_exe()
		.expect("failed to get current executable path");

	// Configure command to run the binary with the hidden flag. Redirect stdio to null 
	// for daemon.
	// NOTE: Consider later adding an option to redirect daemon logs to a file or
	// NOTE: another terminal window.
	let mut command = std::process::Command::new(exe);
	command
	   // Trigger hidden flag to call real daemon.
		.arg(format!("--{}", INTERNAL_DAEMON_INIT_FLAG))
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null());
	
	// Detach from controlling terminal using standard new session strategy. 
	// std::process's implementation of setsid() is still nightly-only feature, 
	// so I use nix crate's alternative.
	unsafe {
		use std::os::unix::process::CommandExt;
		command.pre_exec(|| {
			nix::unistd::setsid().map_err(
				|e: nix::errno::Errno| 
				std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
			).expect("failed to create new session");
			Ok(())
		});
	}

	// Run the spawn command.
	let child = command.spawn()
		.expect("failed to spawn daemon process");

	// TODO: Consider implementing some waiting strategy for the daemon to be 
	// TODO: fully initialized, e.g. by waiting for the PID file or socket file to 
	// TODO: be created, or something else.

	println!("Daemon started with pid {}", child.id());
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
		// TODO: Handle error brought by Err(_)
		Err(_) => {
			println!("Failed to stop daemon with PID {}", pid);
			return;
		}
	}

	// TODO: Implement waiting for process termination with timeout and forced kill 
	// TODO: if it doesn't terminate gracefully.
}

// Check if daemon is running and return its PID if it is. 
// Also performs cleanup of stale PID files and sockets.
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
			if let Some(_) = pid {
				eprintln!("");
				todo!(
					"daemon is running, but socket file is not found\n\
					 Consider implementing some recovery strategy for this case, \
					 e.g. recreate socket file or something. May happen that the process \
					 detected is not daemon at all, but some other process with the same PID"
				);
			}
		}
	}

	pid
}

// Helpers.

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

// TODO: This should be taken in account: if sending a null signal fails with 
// TODO: the error ESRCH, then we know the process doesn’t exist. If the call fails 
// TODO: with the error EPERM (meaning the process exists, but we don’t have permission 
// TODO: to send a signal to it) or succeeds (meaning we do have permission to send
// TODO: a signal to the process), then we know that the process exists.
fn is_process_alive(pid: nix::unistd::Pid) -> bool {
	match nix_signal::kill(pid, None) {
		Ok(()) => true,
		// Process exists, but we don’t have permission to send a signal to it.
		Err(nix::errno::Errno::EPERM) => 
			todo!(
				"Daemon process with PID {} exists but we don't have permission to 
				 signal it. Consider implementing some strategy for this case, e.g. 
				 checking if it's actually our daemon process or not", pid
			),
		// Process doesn't exist.
		Err(nix::errno::Errno::ESRCH) => false,
		// TODO: Handle error brought by Err(_).
		Err(_) => false,
	}
}
