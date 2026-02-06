use crate::types::IpcMessage;
use std::io::{Read, Write};

const SOCKET_PATH: &str = "/tmp/chat_daemon.sock";

pub fn socket_path() -> &'static str {
  SOCKET_PATH
}

pub fn send_message<W: Write>(writer: &mut W, msg: &IpcMessage) -> Result<(), Box<dyn std::error::Error>> {
  let json = serde_json::to_string(msg)?;
  let data = format!("{}\n", json);
  writer.write_all(data.as_bytes())?;
  writer.flush()?;
  Ok(())
}

pub fn receive_message<R: Read>(reader: &mut R) -> Result<Option<IpcMessage>, Box<dyn std::error::Error>> {
  let mut buffer = [0u8; 8192];
  let n = reader.read(&mut buffer)?;

  if n == 0 {
    return Ok(None);
  }

  let data = String::from_utf8_lossy(&buffer[..n]);
  let trimmed = data.trim();

  if trimmed.is_empty() {
    return Ok(None);
  }

  let msg: IpcMessage = serde_json::from_str(trimmed)?;
  Ok(Some(msg))
}

pub struct MessageStream<R: Read> {
  reader: R,
  buffer: String,
}

impl<R: Read> MessageStream<R> {
  pub fn new(reader: R) -> Self {
    MessageStream {
      reader,
      buffer: String::new(),
    }
  }

  pub fn next_message(&mut self) -> Result<Option<IpcMessage>, Box<dyn std::error::Error>> {
    loop {
      if let Some(newline_pos) = self.buffer.find('\n') {
        let line = self.buffer.drain(..=newline_pos).collect::<String>();
        let trimmed = line.trim();
        if !trimmed.is_empty() {
          let msg = serde_json::from_str(trimmed)?;
          return Ok(Some(msg));
        }
      }

      let mut chunk = [0u8; 4096];
      let n = self.reader.read(&mut chunk)?;

      if n == 0 {
        if self.buffer.is_empty() {
          return Ok(None);
        }
        let trimmed = self.buffer.trim();
        if !trimmed.is_empty() {
          let msg = serde_json::from_str(trimmed)?;
          self.buffer.clear();
          return Ok(Some(msg));
        }
        return Ok(None);
      }

      self.buffer.push_str(&String::from_utf8_lossy(&chunk[..n]));
    }
  }
}
