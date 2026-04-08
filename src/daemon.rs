// TODO: Implement error handling.

mod p2p_service;
mod pidfile;

use nix::{
	sys::signal, 
	unistd::{Pid, setsid as nix_setsid},
};
use std::{
	process, thread, time, 
	os::unix, sync::atomic, time::Duration,
	// Temporary (for temp signal handling method):
	sync::atomic::{AtomicBool, Ordering},
	sync::Arc,
};
use signal_hook::{iterator::Signals, consts::signal::SIGTERM};

// ========================================================================
// Signal handling // TODO: Implement forced termination after timeout or 2-nd SIGTERM idk
// ========================================================================

// Temporary signal handling method (later async with Tokio will be introduced):
fn handle_signals() -> Arc<AtomicBool> {
	let term_flag = Arc::new(AtomicBool::new(false));
	let thread_tf = Arc::clone(&term_flag);

	let mut signals = Signals::new(&[SIGTERM])
		.expect("failed to create signal handler");

	thread::spawn(move || {
		for sig in signals.forever() { //  TDOO: use signals.handle() to close the iterator 
																				//	(now it is not necessary because the only implemented 
																				//	signal should terminate the whole process)
			match sig {
				SIGTERM => {
					thread_tf.store(true, Ordering::SeqCst);
				},
				_ => unreachable!(),
			}
		}
	});
	
	term_flag
}

// ========================================================================
// Daemon body
// ========================================================================

pub fn daemon() {
	pidfile::write_pid_file(pidfile::this_proc_id());

	let term_flag = handle_signals();

	println!(
		"Daemon initialized with PID {}", 
		pidfile::this_proc_id()
	);

	while !term_flag.load(Ordering::SeqCst) {
		thread::sleep(Duration::from_secs(1)); // Placeholder for actual daemon work
		println!("Daemon is running..."); // Placeholder for actual daemon work
	}

	println!("Received termination signal, shutting down daemon...");
	pidfile::remove_pid_file();
}

// ========================================================================
// Daemon control functions
// ========================================================================

/* Spawning daemon
		This function is called by CLI when user use `daemon start` commmand.
*/
pub fn start_daemon() {
	if let Some(pid) = pidfile::read_pid_file() {
		if is_process_alive(pid) {
			println!("Daemon is already running.");
			return;
		} else {
			pidfile::remove_pid_file();
			println!("Found stale PID file with PID {} and removed it, starting daemon.", pid);
		}
	} /* TODO: If PID exists, it doesn't necessarily mean the process is precisely our daemon. 
	Implement this check. Stronger long-term design for the future: 
	pair this with a lockfile or Unix socket. */ 

	let exe = std::env::current_exe()
		.expect("failed to get current executable path");

	let mut command = process::Command::new(exe);
	command
		.arg("daemon")
		.arg("start")
		.arg("--initialize") // Trigger hidden flag to call real daemon
		.stdin(process::Stdio::null())
		.stdout(process::Stdio::null())
		.stderr(process::Stdio::null());
	
	/* Detach from controlling terminal using standard new session strategy. std::process's 
	implementation of setsid() is still nightly-only feature, so use nix crate's alternative.*/
	unsafe {
		use unix::process::CommandExt; // for pre_exec()
		command.pre_exec(|| {
			nix_setsid().map_err(
				|err: nix::errno::Errno| std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
			).expect("failed to create new session");
			Ok(())
		});
	}

	let child = command.spawn()
		.expect("failed to spawn daemon process");
	println!("Daemon started with pid {}", child.id());
}

pub fn stop_daemon() {
	let pid = match pidfile::read_pid_file() {
		Some(pid) => {
			if !is_process_alive(pid) {
				pidfile::remove_pid_file();
				println!("Found stale PID file with PID {} and removed it.", pid);
				return;
			}
			pid
		},
		None => {
			println!("Daemon is not running (PID file not found).");
			return;
		}
	};

	match signal::kill(pid, signal::Signal::SIGTERM) {
		Ok(()) => {
			println!("Sent SIGTERM to daemon with PID {}.", pid);
		},
		Err(nix::errno::Errno::ESRCH) => {
			pidfile::remove_pid_file();
			println!("Daemon process with PID {} does not exist. Removed stale PID file.", pid);
			return;
		},
		Err(nix::errno::Errno::EPERM) => {
			println!("Permission denied while stopping daemon with PID {}.", pid);
			return;
		},
		Err(err) => {
			println!("Failed to stop daemon with PID {}.", pid);
			return;
		}
	}
	// TODO: Implement waiting for process termination with timeout and forced kill 
	// if it doesn't terminate gracefully.
}

/* Cases:
	 1 — Daemon is running
	 0 — Daemon is not running / daemon is shutting down / daemon is starting up
*/
pub fn daemon_status() -> bool { // TODO: Implement something smarter than `bool`
	match pidfile::read_pid_file() {
		Some(pid) => {
			if is_process_alive(pid) {
				// println!("Daemon is running with PID {}.", pid);
				true
			} else {
				pidfile::remove_pid_file();
				// println!("Found stale PID file with PID {} and removed it. \
									// Daemon is not running.", pid);
				false
			}
		},
		None => {
			// println!("Daemon is not running.");
		  false
		}
	}
}

// ========================================================================
// Helpers
// ========================================================================

fn is_process_alive(pid: Pid) -> bool {
	match signal::kill(pid, None) {
		Ok(()) => true,
		Err(nix::errno::Errno::EPERM) => true,
		Err(nix::errno::Errno::ESRCH) => false,
		Err(_) => false,
	}
	/* TODO: This should be taken in account: If sending a null signal fails with the error ESRCH, 
	then we know the process doesn’t exist. If the call fails with the error EPERM 
	(meaning the process exists, but we don’t have permission to send a signal to it) 
	or succeeds (meaning we do have permission to send a signal to the process), 
	then we know that the process exists. */
}
