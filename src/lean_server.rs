use std::{
  io::Error as IoError,
  path::{Path, PathBuf},
};

use anyhow::{Context, Error};
use bytes::{Buf, BytesMut};
use mkutils::{IntoStream, Process, Utils};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value as Json;
use tokio::{
  io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
  process::{Child, ChildStderr, ChildStdin, ChildStdout},
  sync::mpsc::{UnboundedReceiver as MpscUnboundedReceiver, UnboundedSender as MpscUnboundedSender},
  task::JoinHandle,
};
use tokio_stream::wrappers::{LinesStream, UnboundedReceiverStream as MpscUnboundedReceiverStream};
use valuable::Valuable;

struct LeanServerStdout {
  buf: BytesMut,
  stdout: ChildStdout,
}

impl LeanServerStdout {
  pub const SEPARATOR: &'static [u8] = b"\r\n\r\n";

  fn new(stdout: ChildStdout) -> Self {
    let buf = BytesMut::new();

    Self { buf, stdout }
  }

  // NOTE: [https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#headerPart]
  #[tracing::instrument(skip_all)]
  async fn next_message(&mut self) -> Result<BytesMut, Error> {
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

struct LeanServerProcess {
  child: Child,
  inputs: MpscUnboundedReceiverStream<Vec<u8>>,
  outputs: MpscUnboundedSender<BytesMut>,
  stdin: ChildStdin,
  stdout: LeanServerStdout,
  stderr: LinesStream<BufReader<ChildStderr>>,
}

impl LeanServerProcess {
  pub const LOG_DIRPATH_ENV_NAME: &'static str = "LEAN_SERVER_LOG_DIR";
  pub const SEPARATOR: &'static [u8] = b"\r\n\r\n";

  pub fn new(
    project_dirpath: &Path,
    log_dirpath: Option<&Path>,
    inputs: MpscUnboundedReceiver<Vec<u8>>,
    outputs: MpscUnboundedSender<BytesMut>,
  ) -> Result<Self, Error> {
    let inputs = inputs.into_stream();
    let (child, stdin, stdout, stderr) = Self::process(&project_dirpath.absolute()?, log_dirpath)?.into_parts();
    let stdout = LeanServerStdout::new(stdout);
    let stderr = BufReader::new(stderr).lines().into_stream();
    let lean_server = Self {
      child,
      inputs,
      outputs,
      stdin,
      stdout,
      stderr,
    };

    lean_server.ok()
  }

  fn process(project_dirpath: &Path, log_dirpath: Option<&Path>) -> Result<Process, Error> {
    let env = log_dirpath.map(|log_dirpath| Self::LOG_DIRPATH_ENV_NAME.pair(log_dirpath));
    let process = Process::new("lake", ["serve"], env, project_dirpath.some())?;

    process.ok()
  }

  async fn write_to_process(&mut self, json_byte_str: &[u8]) -> Result<(), IoError> {
    let content_length_byte_str = json_byte_str.len().to_string().into_bytes();

    self.stdin.write_all(b"Content-Length: ").await?;
    self.stdin.write_all(&content_length_byte_str).await?;
    self.stdin.write_all(Self::SEPARATOR).await?;
    self.stdin.write_all(json_byte_str).await?;
    self.stdin.flush().await?;

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  pub async fn run(mut self) -> Result<(), Error> {
    loop {
      tokio::select! {
        input_byte_str_res = self.inputs.next_item_async() => self.write_to_process(&input_byte_str_res?).await?,
        output_byte_str_res = self.stdout.next_message() => self.outputs.send(output_byte_str_res?)?,
        message_res = self.stderr.next_item_async() => tracing::warn!(message = message_res??, "stderr message"),
        exit_status_res = self.child.wait() => tracing::warn!(exit_status = %exit_status_res?, "lean server process ended"),
      }
    }
  }
}

#[allow(dead_code)]
pub struct LeanServer {
  inputs: MpscUnboundedSender<Vec<u8>>,
  outputs: MpscUnboundedReceiverStream<BytesMut>,
  project_dirpath: PathBuf,
  process_handle: JoinHandle<Result<(), Error>>,
}

impl LeanServer {
  pub const LOG_DIRPATH_ENV_NAME: &'static str = LeanServerProcess::LOG_DIRPATH_ENV_NAME;

  pub async fn new(project_dirpath: &Path, log_dirpath: Option<&Path>) -> Result<Self, Error> {
    // NOTE-97a211
    let project_dirpath = project_dirpath.absolute()?.into_owned();
    let (inputs, process_inputs) = tokio::sync::mpsc::unbounded_channel();
    let (process_outputs, outputs) = tokio::sync::mpsc::unbounded_channel();
    let outputs = outputs.into_stream();
    let process_handle = LeanServerProcess::new(&project_dirpath, log_dirpath, process_inputs, process_outputs)?
      .run()
      .spawn_task();
    let mut lean_server = Self {
      inputs,
      outputs,
      project_dirpath,
      process_handle,
    };

    lean_server.init().await?;

    lean_server.ok()
  }

  async fn init(&mut self) -> Result<(), Error> {
    let initialize_request = crate::messages::initialize::request(&self.project_dirpath, std::process::id())?;

    self.send(initialize_request)?;

    // TODO: note this should be the received initialize request response, and i
    // should not send any more messages until this is received
    let response = self.recv::<Json>().await?;

    tracing::info!(response = response.as_value(), "initial lean server response");

    ().ok()
  }

  pub fn send<T: Serialize>(&mut self, value: T) -> Result<(), Error> {
    let json_byte_str = value.json_byte_str()?;

    self.inputs.send(json_byte_str)?;

    tracing::info!(json = value.json()?.as_value(), "sent message");

    ().ok()
  }

  pub async fn recv<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
    self.outputs.next_item_async().await?.json_from_byte_str::<T>()?.ok()
  }
}
