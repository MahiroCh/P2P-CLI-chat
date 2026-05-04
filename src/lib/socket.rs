//! Socket file management.

mod error;

pub use error::{Error, ErrorKind};
use error::{FrameTooLargeData, Result};

use std::{fs, path::Path};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{UnixListener as TokioUnixListener, UnixStream as TokioUnixStream},
};

// == Socket file management ==

pub const MAX_FRAME_BYTES: u32 = 64 * 1024;

pub fn create(path: &Path) -> Result<TokioUnixListener> {
  match cleanup(path) {
    Ok(()) => {}
    Err(err) if matches!(err.kind(), ErrorKind::RemoveParentDir) => {
      // It's bad because this error is caused not because dir is not empty or
      // not found (see cleanup() implementation) but because of some other I/O,
      // but we can still try to continue and bind the socket, and if it fails,
      // we'll report that error instead.
    }
    Err(err) => return Err(err),
  }

  let parent = path
    .parent()
    .ok_or_else(|| Error::from(ErrorKind::ParentDirInvalid))?;

  fs::create_dir_all(parent)
    .map_err(|source| Error::new(ErrorKind::CreateParentDir, source))?;
  let listener = TokioUnixListener::bind(path)
    .map_err(|source| Error::new(ErrorKind::BindListener, source))?;

  Ok(listener)
}

pub fn cleanup(path: &Path) -> Result<()> {
  match fs::remove_file(path) {
    Ok(()) => {}
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
    Err(source) => {
      return Err(Error::new(ErrorKind::RemoveSocketFile, source));
    }
  }

  if let Some(parent) = path.parent() {
    match fs::remove_dir(parent) {
      Ok(()) => {}
      Err(err)
        if matches!(
          err.kind(),
          std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
        ) => {}
      Err(source) => {
        return Err(Error::new(ErrorKind::RemoveParentDir, source));
      }
    }
  }

  Ok(())
}

// NOTE: This function is not cancel safe. It should only be called in contexts
// NOTE: where is is guaranteed to complete.
pub async fn write_data(socket: &mut TokioUnixStream, message: &str) -> Result<()> {
  let msg_as_bytes = message.as_bytes();
  let msg_byte_len = msg_as_bytes.len() as u32;

  if msg_byte_len > MAX_FRAME_BYTES {
    return Err(Error::new(
      ErrorKind::FrameTooLarge,
      FrameTooLargeData {
        given: msg_byte_len,
        max: MAX_FRAME_BYTES,
      },
    ));
  }

  match socket.write_u32(msg_byte_len).await {
    Ok(()) => {}
    Err(source) if is_connection_abort_like(&source) => {
      return Err(Error::from(ErrorKind::ConnectionAborted));
    }
    Err(source) => {
      return Err(Error::new(ErrorKind::WriteToSocket, source));
    }
  }

  match socket.write_all(msg_as_bytes).await {
    Ok(()) => {}
    Err(source) if is_connection_abort_like(&source) => {
      return Err(Error::from(ErrorKind::ConnectionAborted));
    }
    Err(source) => {
      return Err(Error::new(ErrorKind::WriteToSocket, source));
    }
  }

  Ok(())
}

// NOTE: This function is not cancel safe. It should only be called in contexts
// NOTE: where is is guaranteed to complete.
pub async fn read_data(socket: &mut TokioUnixStream) -> Result<String> {
  let msg_byte_len = match socket.read_u32().await {
    Ok(len) => len,
    Err(source) if is_connection_abort_like(&source) => {
      return Err(Error::from(ErrorKind::ConnectionAborted));
    }
    Err(source) => {
      return Err(Error::new(ErrorKind::ReadFromSocket, source));
    }
  };

  if msg_byte_len > MAX_FRAME_BYTES {
    return Err(Error::new(
      ErrorKind::FrameTooLarge,
      FrameTooLargeData {
        given: msg_byte_len,
        max: MAX_FRAME_BYTES,
      },
    ));
  }

  let mut msg_as_bytes = vec![0u8; msg_byte_len as usize];
  match socket.read_exact(&mut msg_as_bytes).await {
    Ok(_) => {}
    Err(source) if is_connection_abort_like(&source) => {
      return Err(Error::from(ErrorKind::ConnectionAborted));
    }
    Err(source) => {
      return Err(Error::new(ErrorKind::ReadFromSocket, source));
    }
  }

  let msg_as_json = String::from_utf8(msg_as_bytes)
    .map_err(|_| Error::from(ErrorKind::InvalidUtf8))?;

  Ok(msg_as_json)
}

// == Helpers ==

// Check if the error is similar to connection abort errors, which can be
// caused by the peer closing the connection.
fn is_connection_abort_like(err: &std::io::Error) -> bool {
  matches!(
    err.kind(),
    std::io::ErrorKind::BrokenPipe
      | std::io::ErrorKind::ConnectionReset
      | std::io::ErrorKind::ConnectionAborted
      | std::io::ErrorKind::UnexpectedEof
  )
}
