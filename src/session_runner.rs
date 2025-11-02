use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use anyhow::Error as AnyhowError;
use mkutils::{IntoStream, Utils};
use serde_json::Value as Json;
use strum::Display;
use tokio::sync::{mpsc::UnboundedReceiver as MpscUnboundedReceiver, oneshot::Sender as OneshotSender};
use tokio_stream::wrappers::UnboundedReceiverStream as MpscUnboundedReceiverStream;
use ulid::Ulid;
use valuable::Valuable;

use crate::{
  commands::SessionCommand,
  lean_server::LeanServer,
  messages::Request as RequestMessage,
  types::{GetPlainGoalsResult, Location, SessionStatus},
};

#[derive(Display)]
enum Request {
  Initialize(OneshotSender<()>),
  GetPlainGoals(OneshotSender<GetPlainGoalsResult>),
  TextDocumentDocumentSymbol,
  TextDocumentDocumentCodeAction,
  TextDocumentFoldingRange,
  LeanRpcConnect,
}

pub struct SessionResult {
  pub id: Ulid,
  pub result: Result<(), AnyhowError>,
}

pub struct SessionRunner {
  id: Ulid,
  lean_server: LeanServer,
  project_dirpath: PathBuf,
  commands: MpscUnboundedReceiverStream<SessionCommand>,
  requests: HashMap<usize, Request>,
}

impl SessionRunner {
  const MANIFEST_FILE_NAME: &'static str = "lake-manifest.json";

  pub fn new(
    id: Ulid,
    commands: MpscUnboundedReceiver<SessionCommand>,
    lean_path: &Path,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<Self, AnyhowError> {
    let commands = commands.into_stream();
    let project_dirpath = Self::project_dirpath(lean_path)?;
    let lean_server = LeanServer::new(&project_dirpath, lean_server_log_dirpath)?;
    let requests = HashMap::default();
    let session_runner = Self {
      id,
      lean_server,
      project_dirpath,
      commands,
      requests,
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

  fn send_request(&mut self, request_message: RequestMessage, request: Request) -> Result<(), AnyhowError> {
    if self.requests.insert(request_message.id, request).is_some() {
      tracing::warn!(id = %request_message.id, "registering request with existing id");
    }

    self.lean_server.send(request_message.json)
  }

  fn initialize(&mut self, sender: OneshotSender<()>) -> Result<(), AnyhowError> {
    let request_message = self.lean_server.initialize_request()?;
    let request = Request::Initialize(sender);

    self.send_request(request_message, request)
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
    let text_document_did_open_notification = messages.text_document_did_open_notification(&text, &uri);
    let text_document_document_symbol_request = messages.text_document_document_symbol_request(&uri);
    let text_document_document_code_action_request = messages.text_document_document_code_action_request(&uri);
    let text_document_folding_range_request = messages.text_document_folding_range_request(&uri);
    let lean_rpc_connect_request = messages.lean_rpc_connect_request(&uri);

    self.lean_server.send(text_document_did_open_notification)?;
    self.send_request(
      text_document_document_symbol_request,
      Request::TextDocumentDocumentSymbol,
    )?;
    self.send_request(
      text_document_document_code_action_request,
      Request::TextDocumentDocumentCodeAction,
    )?;
    self.send_request(text_document_folding_range_request, Request::TextDocumentFoldingRange)?;
    self.send_request(lean_rpc_connect_request, Request::LeanRpcConnect)?;

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  fn get_plain_goals(
    &mut self,
    sender: OneshotSender<GetPlainGoalsResult>,
    location: &Location,
  ) -> Result<(), AnyhowError> {
    let uri = location.filepath.to_uri()?;
    let request_message =
      self
        .lean_server
        .messages()
        .lean_rpc_get_plain_goals_request(&uri, location.line, location.character);
    let request = Request::GetPlainGoals(sender);

    self.send_request(request_message, request)
  }

  fn get_status(&self) -> SessionStatus {
    let id = self.id;
    let process = self.lean_server.process_status();

    SessionStatus { id, process }
  }

  #[tracing::instrument(skip_all)]
  async fn process_command(&mut self, session_command: SessionCommand) -> Result<(), AnyhowError> {
    match session_command {
      SessionCommand::Initialize { sender } => self.initialize(sender),
      SessionCommand::OpenFile { sender, filepath } => self.open_file(&filepath).await.send_to_oneshot(sender),
      SessionCommand::GetPlainGoals { sender, location } => self.get_plain_goals(sender, &location),
      SessionCommand::GetStatus { sender } => self.get_status().send_to_oneshot(sender),
    }
  }

  fn remove_associated_request(&mut self, response: &Json) -> Option<Request> {
    #[allow(clippy::cast_possible_truncation)]
    let id = response.get("id")?.as_u64()? as usize;
    let request = self.requests.remove(&id)?;

    request.some()
  }

  fn process_response(&mut self, response: &Json) -> Result<(), AnyhowError> {
    let Some(request) = self.remove_associated_request(response) else {
      return tracing::warn!(
        received_message = response.as_value(),
        "received message without matching request"
      )
      .ok();
    };

    tracing::info!(response = response.as_value(), %request, "received response for request");

    match request {
      Request::Initialize(sender) => {
        ().send_to_oneshot(sender)?;
        self
          .lean_server
          .send(self.lean_server.messages().initialized_notification())?;
      }
      Request::GetPlainGoals(sender) => response
        .to_value_from_value::<GetPlainGoalsResult>()?
        .send_to_oneshot(sender)?,
      _ => (),
    }

    ().ok()
  }

  // TODO-8dffbb
  #[tracing::instrument(skip_all)]
  async fn result(mut self) -> Result<(), AnyhowError> {
    loop {
      tokio::select! {
        session_command_res = self.commands.next_item_async() => self.process_command(session_command_res?).await?,
        json_response_res = self.lean_server.recv::<Json>() => self.process_response(&json_response_res?)?,
      }
    }
  }

  pub async fn run(self) -> SessionResult {
    let id = self.id;
    let result = self.result().await;

    SessionResult { id, result }
  }
}
