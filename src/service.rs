// TODO: Implement error handling.

use nix::
{
	sys::signal, 
	unistd
};

use std::
{
	process, env, fs, thread, path, time, 
	os::unix, sync::atomic
};

// ========================================================================
// For thread-safe termination handling
// ========================================================================

// static STOP_REQUESTED: atomic::AtomicBool = atomic::AtomicBool::new(false);

// extern "C" fn handle_sigterm(_: i32) {
// 	STOP_REQUESTED.store(true, atomic::Ordering::SeqCst);
// }

// ========================================================================
// Daemon control functions
// ========================================================================

/* Spawning daemon
		This function is called by CLI when user use `daemon start` commmand.
*/
pub fn start_daemon() {
	if let Some(pid) = read_pid_file() {
		if is_process_alive(pid) {
			println!("Daemon already running, use `daemon connect` to connect.");
			return;
		} else {
			remove_pid_file();
			println!("Found stale PID file with PID {} and removed it, starting daemon.", pid);
		}
	} /* TODO: If PID exists, it doesn't necessarily mean the process is precisely our daemon. 
	Implement this check. Stronger long-term design for the future: 
	pair this with a lockfile or Unix socket. */ 

	let exe = env::current_exe()
		.expect("failed to get current executable path");
	let mut command = process::Command::new(exe);
	command
		.arg("daemon")
		.arg("start")
		.arg("--initialize")
		.stdin(process::Stdio::null())
		.stdout(process::Stdio::inherit()) // TODO: Redirect to null when don't need debugging anymore
		.stderr(process::Stdio::null());

	/* Detach from controlling terminal using standard new session strategy. std::process's 
	implementation of setsid() is still nightly-only feature, so use nix crate's alternative.*/
	unsafe {
		use unix::process::CommandExt; // For pre_exec()
		command.pre_exec(|| {
			nix::unistd::setsid().map_err(
				|err: nix::errno::Errno| std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
			).expect("failed to create new session");
			Ok(())
		});
	}

	let child = command.spawn()
		.expect("failed to spawn daemon process");
	println!("Daemon started with pid {}", child.id());
}

pub fn connect_daemon() { // TODO: Implement function
	println!("Connecting to daemon... (not implemented yet)");
}

pub fn stop_daemon() { // TODO: Implement function
	println!("Stopping daemon... (not implemented yet)");
}

pub fn restart_daemon() { // TODO: Implement function
	println!("Restarting daemon... (not implemented yet)");
}

pub fn daemon_status() { // TODO: Implement function
	println!("Checking daemon status... (not implemented yet)");
}

// ========================================================================
// Daemon itself
// ========================================================================

const DAEMON_NAME: &str = "p2pchat";

pub fn daemon() {
	write_pid_file(process::id());

	println!(
		"Daemon is running with PID {} \
		(but it will now immediately die after this message.", 
		process::id()
	);

	remove_pid_file();
}

// ========================================================================
// PID file operations
// ========================================================================

fn pid_file_path() -> path::PathBuf {
	if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
		return path::PathBuf::from(runtime_dir).join(format!("{DAEMON_NAME}.pid"));
	}

	let home = env::var_os("HOME")
		.expect("$HOME environment variable is not set");
	path::PathBuf::from(home).join(format!(".local/run/{DAEMON_NAME}.pid"))
}

fn write_pid_file(pid: u32) {
	let path = pid_file_path();

	let parent = path.parent()
		.expect("PID file path must have a parent directory (at least /run or ~/.local/...)");
	fs::create_dir_all(parent)
		.expect("failed to create PID file parent directory");

	fs::write(path, pid.to_string())
		.expect("failed to write PID file");
}

fn remove_pid_file() {
	match fs::remove_file(pid_file_path()) {
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => (),
		Err(err) => panic!("failed to remove PID file: {}", err),
		_ => ()
	}
}

fn read_pid_file() -> Option<u32> {
	let path = pid_file_path();

	let file_content = match fs::read_to_string(&path) {
		Ok(c) => c,
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
		Err(err) => panic!("failed to read PID file: {}", err),
	};
	let pid = file_content.trim().parse::<u32>()
		.expect("failed to parse PID as u32");
	
	Some(pid)
}

// ========================================================================
// Helpers
// ========================================================================

fn is_process_alive(pid: u32) -> bool {
	signal::kill(nix::unistd::Pid::from_raw(pid as i32), None) == Ok(())
	/* TODO: This should be taken in account: If sending a null signal fails with the error ESRCH, 
	then we know the process doesn’t exist. If the call fails with the error EPERM 
	(meaning the process exists, but we don’t have permission to send a signal to it) 
	or succeeds (meaning we do have permission to send a signal to the process), 
	then we know that the process exists. */
}
