use std::path::Path;

use anyhow::Error;
use bytes::{Buf, BytesMut};
use serde::Serialize;
use serde_json::Value as Json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{process::Process, utils::Utils};

pub struct LeanServer {
  process: Process,
  stdout_buf: BytesMut,
}

impl LeanServer {
  pub const LOG_DIRPATH_ENV_NAME: &'static str = "LEAN_SERVER_LOG_DIR";

  const SEPARATOR: &'static [u8] = b"\r\n\r\n";

  pub fn new(project_dirpath: &Path, log_dirpath: Option<&Path>) -> Result<Self, Error> {
    let process = Self::process(project_dirpath, log_dirpath)?;
    let stdout_buf = BytesMut::new();
    let lean_server = Self { process, stdout_buf };

    lean_server.ok()
  }

  fn process(project_dirpath: &Path, log_dirpath: Option<&Path>) -> Result<Process, Error> {
    let env = log_dirpath.map(|log_dirpath| Self::LOG_DIRPATH_ENV_NAME.pair(log_dirpath));
    let process = Process::new("lake", ["serve"], env, project_dirpath.some())?;

    process.ok()
  }

  async fn next_message(&mut self) -> Result<Json, Error> {
    let (content_begin_idx, content_end_idx) = loop {
      if let Some((separator_begin_idx, separator_end_idx)) = self.stdout_buf.substr_interval(Self::SEPARATOR) {
        let Some((_space_begin_idx, space_end_idx)) =
          // TODO-4eef0b
          self.stdout_buf[..separator_begin_idx].substr_interval(b" ")
        else {
          anyhow::bail!("invalid header")
        };
        let content_length = self.stdout_buf[space_end_idx..separator_begin_idx]
          .as_utf8()?
          .parse::<usize>()?;
        let content_end_idx = separator_begin_idx + content_length;

        break (separator_end_idx, content_end_idx);
      }

      self.process.stdout_mut().read_buf(&mut self.stdout_buf).await?;
    };

    // NOTE: read bytes until there are enough for the response content
    while self.stdout_buf.len() < content_end_idx {
      self.process.stdout_mut().read_buf(&mut self.stdout_buf).await?;
    }

    let content_byte_str = &self.stdout_buf[content_begin_idx..content_end_idx];
    let message = serde_json::from_slice::<Json>(content_byte_str)?;

    // NOTE: pop bytes from beginning of buffer
    self.stdout_buf.advance(content_end_idx);

    message.ok()
  }

  pub async fn send<T: Serialize>(&mut self, value: T) -> Result<(), Error> {
    let stdin = self.process.stdin_mut();
    let json_str = value.json_string()?;
    let json_byte_str = json_str.as_bytes();
    let content_length_byte_str = json_byte_str.len().to_string().into_bytes();

    stdin.write_all(b"Content-Length: ").await?;
    stdin.write_all(&content_length_byte_str).await?;
    stdin.write_all(Self::SEPARATOR).await?;
    stdin.write_all(json_byte_str).await?;
    stdin.flush().await?;

    tracing::info!(message = "sent message", json_str);

    ().ok()
  }

  pub async fn run(&mut self) -> Result<(), Error> {
    let initialize_request = serde_json::json!({});

    self.send(initialize_request).await?;

    let response = self.next_message().await?;
    let exit_status = self.process.run().await?;
    let stdout = self.process.stdout_mut().read_string().await?;
    let stderr = self.process.stderr_mut().read_string().await?;

    tracing::info!(message = "lean server process complete", %response, stdout, stderr, %exit_status);

    ().ok()
  }
}
