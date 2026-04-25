//! Socket file management.

use std::{
	fs, path::PathBuf
};
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt},
	net::{UnixListener as TokioUnixListener, UnixStream as TokioUnixStream},
};

// Define a maximum frame size for socket communication between CLI and Daemon
// to prevent potential DoS attacks or resource exhaustion.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

pub fn create_file(path: &PathBuf) -> TokioUnixListener {
  remove_file(path);

	let parent = path.parent()
		.expect("socket file path must have a parent directory (at least /run or ~/.cache)");
	fs::create_dir_all(parent)
		.expect("failed to create socket file parent directory");

	let listener = TokioUnixListener::bind(path)
		.expect("failed to bind to socket file");

	listener
}

pub fn remove_file(path: &PathBuf) {
	match fs::remove_file(path) {
		Ok(()) => {},
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => {},
		Err(err) => panic!("failed to remove socket file: {}", err)
	}

	if let Some(parent) = path.parent() {
		match fs::remove_dir(parent) {
			Ok(()) => {},
			Err(err) 
				if err.kind() == std::io::ErrorKind::NotFound
        || err.kind() == std::io::ErrorKind::DirectoryNotEmpty => {},
			Err(_) => {
				todo!(
					"failed to remove socket file parent directory\n\
					 Something unexpected happened, and if it is permission issue, \
           consider implementing some behavior for this case."
				);
			}
		}
	}
}

pub async fn connect_from_cli(path: &PathBuf) -> Option<TokioUnixStream> {
	match TokioUnixStream::connect(path).await {
		Ok(socket) => Some(socket),
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
		Err(err) => 
			todo!(
				"failed to connect to socket file: {}\n\
				This could be because the daemon is not running, or there are permission \
				issues with the socket file.", err
			)
	}
}

pub async fn write_data(socket: &mut TokioUnixStream, message: &str) -> Result<(), std::io::Error> {
	let msg_as_bytes = message.as_bytes();
	let msg_byte_len = msg_as_bytes.len() as u32;

	match socket.write_u32(msg_byte_len).await {
		Ok(_) => {},
		Err(err)
			if err.kind() == std::io::ErrorKind::BrokenPipe
			|| err.kind() == std::io::ErrorKind::ConnectionReset
			|| err.kind() == std::io::ErrorKind::ConnectionAborted => {
			return Err(
				std::io::Error::new(
					std::io::ErrorKind::ConnectionAborted,
					"connection aborted by CLI client"
				)
			);
		},
		// TODO: Handle errors.
		Err(_) => panic!("failed to write message length to socket"),
	}
	
	match socket.write_all(msg_as_bytes).await {
		Ok(_) => {},
		Err(err)
			if err.kind() == std::io::ErrorKind::BrokenPipe
			|| err.kind() == std::io::ErrorKind::ConnectionReset
			|| err.kind() == std::io::ErrorKind::ConnectionAborted  => {
			return Err(
				std::io::Error::new(
					std::io::ErrorKind::ConnectionAborted,
					"connection aborted by CLI client"
				)
			);
		},
		// TODO: Handle errors.
		Err(_) => panic!("failed to write message to socket"),
	}

	Ok(())
}

// TODO: Reading methods are not cancel safe in here.
pub async fn read_data(socket: &mut TokioUnixStream) -> Result<String, std::io::Error> {
		let msg_byte_len = match socket.read_u32().await {
			Ok(len) => len,
			Err(err) 
				if err.kind() == std::io::ErrorKind::BrokenPipe
				|| err.kind() == std::io::ErrorKind::ConnectionReset
				|| err.kind() == std::io::ErrorKind::ConnectionAborted
				|| err.kind() == std::io::ErrorKind::UnexpectedEof => {
				return Err(
					std::io::Error::new(
						std::io::ErrorKind::ConnectionAborted, 
						"connection aborted by CLI client"
					)
				);
			},
			// TODO: Handle errors.
			Err(_) => panic!("failed to read message length from socket"),
		};

		if msg_byte_len as usize > MAX_FRAME_BYTES {
			return Err(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"socket frame exceeds maximum allowed size"
			));
		}

		let mut msg_as_bytes = vec![0u8; msg_byte_len as usize];
		match socket.read_exact(&mut msg_as_bytes).await {
			Ok(_) => {},
			Err(err) 
				if err.kind() == std::io::ErrorKind::BrokenPipe
				|| err.kind() == std::io::ErrorKind::ConnectionReset
				|| err.kind() == std::io::ErrorKind::ConnectionAborted
				|| err.kind() == std::io::ErrorKind::UnexpectedEof => {
				return Err(
					std::io::Error::new(
						std::io::ErrorKind::ConnectionAborted, 
						"connection aborted by CLI client"
					)
				);
			},
			// TODO: Handle errors.
			Err(_) => panic!("failed to read message from socket"),
		}

		let msg_as_json = String::from_utf8(msg_as_bytes)
			.expect("received non-UTF8 message from socket");

	Ok(msg_as_json)
}
