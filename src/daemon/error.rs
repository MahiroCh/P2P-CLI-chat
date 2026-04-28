//! Daemon top-level errors.

use p2p_chat::define_error;

define_error!();

// == Error kinds ==

#[allow(dead_code)]
#[derive(thiserror::Error, Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ErrorKind {
	// Initialization errors.
	#[error("failed to create PID file")]
	PidFileCreationFailed,
	#[error("failed to create socket file")]
	SocketCreationFailed,
	#[error("daemon signal handler failed to initialize")]
	SignalHandlerFailed,

	// Runtime errors.
	#[error("daemon listener failed to accept a connection")]
	SocketAcceptFailed,
}
