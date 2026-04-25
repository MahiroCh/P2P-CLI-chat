//! Daemon process for p2p chat application.

pub mod control;

use p2p_chat::{
	pid,
	socket,
};

use tokio::{
	signal::unix::{signal as tokio_signal, SignalKind}
};

// Shutdown handling.

struct DaemonCleanupGuard;
impl Drop for DaemonCleanupGuard {
	fn drop(&mut self) {
		pid::remove_file(&control::pid_file_path());
		socket::remove_file(&control::socket_file_path());
	}
}

// Driver code.

pub fn run() {
	let _guard = DaemonCleanupGuard;
	
	let pid_fp = control::pid_file_path();
	pid::create_file(&pid_fp);
	pid::write_to_file(&pid_fp, pid::this_proc_pid());

	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let mut sigterm = tokio_signal(SignalKind::terminate())
			.expect("failed to install Tokio SIGTERM handler");
		let socket_fp = control::socket_file_path();
		let socket_listener = socket::create_file(&socket_fp);

		// TODO: Make this truly asynchronous, e.g. by spawning a task and so on.
		'daemon_loop: loop {
			tokio::select! {
				_ = sigterm.recv() => {
					println!("Received SIGTERM; shutting down daemon");
					break 'daemon_loop;
				},

				accepted = socket_listener.accept() => {
					let (mut stream, _) = accepted
						.expect("failed to accept connection");

					loop {
						tokio::select! {
							_ = sigterm.recv() => {
								println!("Received SIGTERM; shutting down daemon");
								break 'daemon_loop;
							},
							
							cmd = socket::read_data(&mut stream) => {
								let cmd = match cmd {
									Ok(value) => value,
									Err(err) if err.kind() == std::io::ErrorKind::ConnectionAborted => break,
									Err(_) => panic!("failed to read message from socket"),
								};

								let answer = String::from(format!("{}", cmd));
								socket::write_data(&mut stream, &answer).await
									.expect("failed to write message to socket");
							}
						}
					}
				}
			}
		}
	});
}
