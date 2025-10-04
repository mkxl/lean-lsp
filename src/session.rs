use std::path::{Path, PathBuf};

use anyhow::Error;
use derive_more::Constructor;
use mkutils::{IntoStream, Utils};
use serde_json::Value as Json;
use tokio::sync::{mpsc::UnboundedSender as MpscUnboundedSender, oneshot::Sender as OneshotSender};
use tokio_stream::wrappers::UnboundedReceiverStream as MpscUnboundedReceiverStream;
use ulid::Ulid;

use crate::lean_server::LeanServer;

pub enum SessionCommand {
  OpenFile {
    sender: OneshotSender<Result<(), Error>>,
    filepath: PathBuf,
  },
}

#[derive(Clone, Constructor)]
pub struct SessionClient {
  id: Ulid,
  sender: MpscUnboundedSender<SessionCommand>,
}

impl SessionClient {
  pub fn id(&self) -> Ulid {
    self.id
  }

  // TODO-8dffbb
  pub async fn open_file(&self, filepath: PathBuf) -> Result<(), Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let open_file_command = SessionCommand::OpenFile { sender, filepath };

    self.sender.send(open_file_command)?;

    receiver.await?
  }
}

pub struct Session {
  id: Ulid,
  lean_server: LeanServer,
  project_dirpath: PathBuf,
  commands: MpscUnboundedReceiverStream<SessionCommand>,
}

impl Session {
  const MANIFEST_FILE_NAME: &'static str = "lake-manifest.json";

  pub async fn new(lean_path: &Path, lean_server_log_dirpath: Option<&Path>) -> Result<(Self, SessionClient), Error> {
    let id = Ulid::new();
    let project_dirpath = Self::project_dirpath(lean_path)?;
    let lean_server = LeanServer::new(&project_dirpath, lean_server_log_dirpath).await?;
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let commands = receiver.into_stream();
    let session = Self {
      id,
      lean_server,
      project_dirpath,
      commands,
    };
    let session_client = SessionClient::new(id, sender);

    tracing::info!(id = %session.id, project_dirpath = %session.project_dirpath.display(), "new session");

    (session, session_client).ok()
  }

  pub fn id(&self) -> Ulid {
    self.id
  }

  fn project_dirpath(lean_path: &Path) -> Result<PathBuf, Error> {
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
  async fn open_file(&mut self, filepath: &Path) -> Result<(), Error> {
    let messages = [
      crate::messages::text_document::did_open_notification(filepath),
      crate::messages::text_document::document_symbol_request(filepath),
      crate::messages::text_document::document_code_action_request(filepath),
      crate::messages::text_document::folding_range_request(filepath),
      crate::messages::lean_rpc::connect_request(filepath),
    ];

    for message in messages {
      self.lean_server.send(message)?;
    }

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  async fn process_command(&mut self, session_command: SessionCommand) -> Result<(), Error> {
    match session_command {
      SessionCommand::OpenFile { sender, filepath } => self.open_file(&filepath).await.send_to_oneshot(sender),
    }
  }

  // TODO-8dffbb
  #[tracing::instrument(skip_all)]
  pub async fn run(mut self) -> Result<(), Error> {
    loop {
      tokio::select! {
        session_command_res = self.commands.next_item_async() => self.process_command(session_command_res?).await?,
        json_res = self.lean_server.recv::<Json>() => tracing::info!(message = %json_res?, "received message"),
      }
    }
  }
}
