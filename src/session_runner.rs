use std::path::{Path, PathBuf};

use anyhow::Error as AnyhowError;
use mkutils::{IntoStream, Utils};
use serde_json::Value as Json;
use tokio::sync::mpsc::UnboundedReceiver as MpscUnboundedReceiver;
use tokio_stream::wrappers::UnboundedReceiverStream as MpscUnboundedReceiverStream;
use ulid::Ulid;

use crate::{commands::SessionCommand, lean_server::LeanServer};

pub struct SessionResult {
  pub id: Ulid,
  pub result: Result<(), AnyhowError>,
}

pub struct SessionRunner {
  id: Ulid,
  lean_server: LeanServer,
  project_dirpath: PathBuf,
  commands: MpscUnboundedReceiverStream<SessionCommand>,
}

impl SessionRunner {
  const MANIFEST_FILE_NAME: &'static str = "lake-manifest.json";

  pub async fn new(
    id: Ulid,
    commands: MpscUnboundedReceiver<SessionCommand>,
    lean_path: &Path,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<Self, AnyhowError> {
    let commands = commands.into_stream();
    let project_dirpath = Self::project_dirpath(lean_path)?;
    let lean_server = LeanServer::new(&project_dirpath, lean_server_log_dirpath).await?;
    let session_runner = Self {
      id,
      lean_server,
      project_dirpath,
      commands,
    };

    tracing::info!(%id, project_dirpath = %session_runner.project_dirpath.display(), "new session");

    session_runner.ok()
  }

  fn project_dirpath(lean_path: &Path) -> Result<PathBuf, AnyhowError> {
    for ancestor_path in lean_path.ancestors() {
      let mut manifest_filepath = ancestor_path.with_file_name(Self::MANIFEST_FILE_NAME);

      if manifest_filepath.is_file() {
        manifest_filepath.pop();

        return manifest_filepath.ok();
      }
    }

    anyhow::bail!("unable to get project dirpath: no manifest file found in ancestor dirpaths");
  }

  #[tracing::instrument(skip_all)]
  async fn open_file(&mut self, filepath: &Path) -> Result<(), AnyhowError> {
    let uri = filepath.to_uri()?;
    let text = filepath
      .open_async()
      .await?
      .buf_reader_async()
      .read_string_async()
      .await?;
    let messages = self.lean_server.messages();
    let messages = [
      messages.text_document_did_open_notification(&text, &uri),
      messages.text_document_document_symbol_request(&uri),
      messages.text_document_document_code_action_request(&uri),
      messages.text_document_folding_range_request(&uri),
      messages.lean_rpc_connect_request(&uri),
    ];

    for message in messages {
      self.lean_server.send(message)?;
    }

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  async fn process_command(&mut self, session_command: SessionCommand) -> Result<(), AnyhowError> {
    match session_command {
      SessionCommand::OpenFile { sender, filepath } => self.open_file(&filepath).await.send_to_oneshot(sender),
      SessionCommand::GetProcessStatus { sender } => self.lean_server.process_status().send_to_oneshot(sender),
    }
  }

  // TODO-8dffbb
  #[tracing::instrument(skip_all)]
  async fn result(mut self) -> Result<(), AnyhowError> {
    loop {
      tokio::select! {
        session_command_res = self.commands.next_item_async() => self.process_command(session_command_res?).await?,
        json_res = self.lean_server.recv::<Json>() => tracing::info!(received_message = %json_res?, "received message"),
      }
    }
  }

  pub fn id(&self) -> Ulid {
    self.id
  }

  pub async fn run(self) -> SessionResult {
    let id = self.id();
    let result = self.result().await;

    SessionResult { id, result }
  }
}
