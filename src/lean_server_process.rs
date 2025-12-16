use std::io::Error as IoError;

use anyhow::{Context, Error as AnyhowError};
use bytes::{Buf, BytesMut};
use camino::Utf8Path;
use futures::StreamExt;
use mkutils::{Process, Utils};
use serde::Serialize;
use tokio::{
  io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt},
  process::{Child, ChildStderr, ChildStdin, ChildStdout},
  task::JoinHandle,
};

use crate::message::Message;

struct LeanServerProcessStdout {
  buf: BytesMut,
  stdout: ChildStdout,
}

impl LeanServerProcessStdout {
  pub const SEPARATOR: &'static [u8] = b"\r\n\r\n";

  fn new(stdout: ChildStdout) -> Self {
    let buf = BytesMut::new();

    Self { buf, stdout }
  }

  // NOTE: [https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#headerPart]
  async fn next_message(&mut self) -> Result<BytesMut, AnyhowError> {
    let (content_begin_idx, content_length) = loop {
      if let Some((separator_begin_idx, separator_end_idx)) = self.buf.substr_interval(Self::SEPARATOR) {
        let (_space_begin_idx, space_end_idx) =
          // TODO-4eef0b
          self.buf[..separator_begin_idx].substr_interval(b" ").context("invalid header")?;
        let content_length = self.buf[space_end_idx..separator_begin_idx]
          .as_utf8()?
          .parse::<usize>()?;

        break (separator_end_idx, content_length);
      }

      self.stdout.read_buf(&mut self.buf).await?;
    };

    // NOTE: skip ahead to the beginning of the response content
    self.buf.advance(content_begin_idx);

    // NOTE: read bytes until there are enough
    while self.buf.len() < content_length {
      self.stdout.read_buf(&mut self.buf).await?;
    }

    // NOTE: pop bytes from beginning of buffer
    let content_byte_str = self.buf.split_to(content_length);

    content_byte_str.ok()
  }
}

pub struct LeanServerProcess {
  child: Child,
  stdin: ChildStdin,
  stdout: LeanServerProcessStdout,
  log_stderr: JoinHandle<Result<(), IoError>>,
}

impl LeanServerProcess {
  pub const LOG_DIRPATH_ENV_NAME: &str = "LEAN_SERVER_LOG_DIR";
  pub const DEFAULT_LAKE_FILEPATH_STR: &str = "lake";

  const CONTENT_LENGTH_HEADER_PREFIX: &[u8] = b"Content-Length: ";
  const LAKE_ARGS: &[&str] = &["serve"];
  const SEPARATOR: &[u8] = b"\r\n\r\n";

  pub fn new(
    lake_filepath: &Utf8Path,
    log_dirpath: Option<&Utf8Path>,
    project_absolute_dirpath: &Utf8Path,
  ) -> Result<Self, AnyhowError> {
    let (child, stdin, stdout, stderr) =
      Self::process(lake_filepath, project_absolute_dirpath, log_dirpath)?.into_parts();
    let stdout = LeanServerProcessStdout::new(stdout);
    let log_stderr = Self::log_stderr(stderr).spawn_task();
    let lean_server_process = Self {
      child,
      stdin,
      stdout,
      log_stderr,
    };

    lean_server_process.ok()
  }

  fn env(log_dirpath: Option<&Utf8Path>) -> Option<(&str, &Utf8Path)> {
    Self::LOG_DIRPATH_ENV_NAME.pair(log_dirpath?).some()
  }

  fn process(
    lake_filepath: &Utf8Path,
    project_absolute_dirpath: &Utf8Path,
    log_dirpath: Option<&Utf8Path>,
  ) -> Result<Process, AnyhowError> {
    let env = Self::env(log_dirpath);
    let process = Process::new(lake_filepath, Self::LAKE_ARGS, env, project_absolute_dirpath)?;

    process.ok()
  }

  async fn log_stderr(stderr: ChildStderr) -> Result<(), IoError> {
    let mut stderr = stderr.buf_reader_async().lines().into_stream();

    while let Some(stderr_message_res) = stderr.next().await {
      tracing::warn!(stderr_message = stderr_message_res?, "stderr message");
    }

    ().ok()
  }

  pub async fn next_message(&mut self) -> Result<Message, AnyhowError> {
    self
      .stdout
      .next_message()
      .await?
      .to_value_from_json_slice::<Message>()?
      .ok()
  }

  pub async fn send<T: Serialize>(&mut self, value: T) -> Result<(), AnyhowError> {
    let json_byte_str = value.to_json_byte_str()?;
    let content_length_byte_str = json_byte_str.len().to_string().into_bytes();

    self.stdin.write_all(Self::CONTENT_LENGTH_HEADER_PREFIX).await?;
    self.stdin.write_all(&content_length_byte_str).await?;
    self.stdin.write_all(Self::SEPARATOR).await?;
    self.stdin.write_all(&json_byte_str).await?;
    self.stdin.flush().await?;

    tracing::info!(json = value.to_json()?.as_valuable(), "sent message");

    ().ok()
  }

  pub async fn kill(&mut self) -> Result<(), IoError> {
    self.log_stderr.abort();
    self.child.kill().await?;

    ().ok()
  }
}
