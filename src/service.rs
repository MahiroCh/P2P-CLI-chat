use std::env;
use std::ffi::CString;
use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt;

use nix::fcntl::OFlag;
use nix::unistd::{execv, fork, pipe2, read, setsid, write, ForkResult};

const DAEMON_TOKEN_ENV: &str = "P2PCHAT_DAEMON_TOKEN";

pub fn start_daemon() -> Result<(), String> {
	let token = make_token();
	env::set_var(DAEMON_TOKEN_ENV, &token);

	let (read_fd, write_fd) = pipe2(OFlag::empty()).map_err(|err| err.to_string())?;
	write_all(&write_fd, token.as_bytes())?;
	drop(write_fd);

	match unsafe { fork() }.map_err(|err| err.to_string())? {
		ForkResult::Parent { .. } => {
			drop(read_fd);
			Ok(())
		}
		ForkResult::Child => {
			setsid().map_err(|err| err.to_string())?;

			let token_fd = read_fd.into_raw_fd();
			let current_exe = env::current_exe().map_err(|err| err.to_string())?;
			let current_exe = CString::new(current_exe.as_os_str().as_bytes()).map_err(|err| err.to_string())?;
			let daemon_mode = CString::new("__daemon-internal").unwrap();
			let token_fd_flag = CString::new("--token-fd").unwrap();
			let token_fd_value = CString::new(token_fd.to_string()).map_err(|err| err.to_string())?;

			let argv = [
				current_exe.as_c_str(),
				daemon_mode.as_c_str(),
				token_fd_flag.as_c_str(),
				token_fd_value.as_c_str(),
			];

			execv(&current_exe, &argv).map_err(|err| err.to_string())?;
			unreachable!()
		}
	}
}

pub fn run_daemon_internal(token_fd: RawFd) -> Result<(), String> {
	let expected_token = env::var(DAEMON_TOKEN_ENV).map_err(|_| "missing daemon token".to_string())?;
	let actual_token = read_all(token_fd)?;

	if actual_token != expected_token {
		return Err("daemon token mismatch".to_string());
	}

	env::remove_var(DAEMON_TOKEN_ENV);
	println!("daemon probe active");
	Ok(())
}

fn make_token() -> String {
	let pid = std::process::id();
	let nanos = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();

	format!("{pid:x}-{nanos:x}")
}

fn write_all(fd: &OwnedFd, mut data: &[u8]) -> Result<(), String> {
	while !data.is_empty() {
		let written = write(fd, data).map_err(|err| err.to_string())?;
		data = &data[written..];
	}

	Ok(())
}

fn read_all(token_fd: RawFd) -> Result<String, String> {
	let token_fd = unsafe { OwnedFd::from_raw_fd(token_fd) };
	let mut buffer = [0u8; 64];
	let mut token = Vec::new();

	loop {
		let bytes_read = read(&token_fd, &mut buffer).map_err(|err| err.to_string())?;
		if bytes_read == 0 {
			break;
		}
		token.extend_from_slice(&buffer[..bytes_read]);
	}

	String::from_utf8(token).map_err(|err| err.to_string())
}
