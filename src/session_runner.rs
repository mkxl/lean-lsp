use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use anyhow::Error as AnyhowError;
use mkutils::{IntoStream, ToValue, Utils};
use serde_json::Value as Json;
use strum::Display;
use tokio::sync::{
  broadcast::Sender as BroadcastSender, mpsc::UnboundedReceiver as MpscUnboundedReceiver,
  oneshot::Sender as OneshotSender,
};
use tokio_stream::wrappers::UnboundedReceiverStream as MpscUnboundedReceiverStream;
use ulid::Ulid;

use crate::{
  commands::SessionCommand,
  lean_server::LeanServer,
  messages::{Id, Message, text_document::INITIAL_TEXT_DOCUMENT_VERSION},
  server::responses::{GetPlainGoalsResponse, HoverFileResponse},
  types::{Location, SessionStatus},
};

#[derive(Display)]
enum Request {
  Initialize(OneshotSender<()>),
  GetPlainGoals(OneshotSender<GetPlainGoalsResponse>),
  Hover(OneshotSender<HoverFileResponse>),
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
  requests: HashMap<Id, Request>,
  notifications: BroadcastSender<Json>,
  open_file_versions: HashMap<PathBuf, usize>,
}

impl SessionRunner {
  const MANIFEST_FILE_NAME: &'static str = "lake-manifest.json";

  pub fn new(
    id: Ulid,
    commands: MpscUnboundedReceiver<SessionCommand>,
    notifications: BroadcastSender<Json>,
    lean_path: &Path,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<Self, AnyhowError> {
    let commands = commands.into_stream();
    let project_dirpath = Self::project_dirpath(lean_path)?;
    let lean_server = LeanServer::new(&project_dirpath, lean_server_log_dirpath)?;
    let requests = HashMap::default();
    let open_file_versions = HashMap::new();
    let session_runner = Self {
      id,
      lean_server,
      project_dirpath,
      commands,
      requests,
      notifications,
      open_file_versions,
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

  fn send_request(&mut self, request_message: Message, request: Request) -> Result<(), AnyhowError> {
    if self.requests.insert(request_message.id.clone(), request).is_some() {
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
  async fn open_file(&mut self, filepath: PathBuf) -> Result<(), AnyhowError> {
    if self.open_file_versions.contains_key(&filepath) {
      anyhow::bail!("file {} is already open", filepath.display());
    }

    let uri = filepath.to_uri()?;
    let text = filepath
      .open_async()
      .await?
      .buf_reader_async()
      .read_string_async()
      .await?;
    let text_document_did_open_notification = Message::text_document_did_open_notification(&text, &uri);
    let text_document_document_symbol_request = Message::text_document_document_symbol_request(&uri);
    let text_document_document_code_action_request = Message::text_document_document_code_action_request(&uri);
    let text_document_folding_range_request = Message::text_document_folding_range_request(&uri);
    let lean_rpc_connect_request = Message::lean_rpc_connect_request(&uri);

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

    self.open_file_versions.insert(filepath, INITIAL_TEXT_DOCUMENT_VERSION);

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  fn change_file(&mut self, filepath: &Path, text: &str) -> Result<(), AnyhowError> {
    let version = self
      .open_file_versions
      .get_mut(filepath)
      .context_path("file is not open", filepath)?;
    let new_version = *version + 1;

    let uri = filepath.to_uri()?;
    let text_document_did_change_notification = Message::text_document_did_change_notification(text, &uri, new_version);

    self.lean_server.send(text_document_did_change_notification)?;

    // only increment the version if the request was successfully sent
    *version += 1;

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  fn close_file(&mut self, filepath: &Path) -> Result<(), AnyhowError> {
    if !self.open_file_versions.contains_key(filepath) {
      anyhow::bail!("file {} is not open", filepath.display());
    }

    let uri = filepath.to_uri()?;
    let text_document_did_close_notification = Message::text_document_did_close_notification(&uri);

    self.lean_server.send(text_document_did_close_notification)?;

    self.open_file_versions.remove(filepath);

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  fn hover_file(&mut self, sender: OneshotSender<HoverFileResponse>, location: &Location) -> Result<(), AnyhowError> {
    let uri = location.filepath.to_uri()?;
    let message = Message::text_document_hover_request(&uri, location.line, location.character);
    let request = Request::Hover(sender);

    self.send_request(message, request)?;

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  fn get_plain_goals(
    &mut self,
    sender: OneshotSender<GetPlainGoalsResponse>,
    location: &Location,
  ) -> Result<(), AnyhowError> {
    let uri = location.filepath.to_uri()?;
    let request_message = Message::lean_rpc_get_plain_goals_request(&uri, location.line, location.character);
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
      SessionCommand::OpenFile { sender, filepath } => self.open_file(filepath).await.send_to_oneshot(sender),
      SessionCommand::ChangeFile { sender, filepath, text } => {
        self.change_file(&filepath, &text).send_to_oneshot(sender)
      }
      SessionCommand::HoverFile { sender, location } => self.hover_file(sender, &location),
      SessionCommand::CloseFile { sender, filepath } => self.close_file(&filepath).send_to_oneshot(sender),
      SessionCommand::GetPlainGoals { sender, location } => self.get_plain_goals(sender, &location),
      SessionCommand::GetStatus { sender } => self.get_status().send_to_oneshot(sender),
    }
  }

  #[tracing::instrument(skip_all, err)]
  fn process_response(&mut self, request: Request, response: &Json) -> Result<(), AnyhowError> {
    tracing::info!(received_response = response.to_value(), %request, "received response for request");

    match request {
      Request::Initialize(sender) => {
        let notification = Message::initialized_notification();

        ().send_to_oneshot(sender)?;
        self.lean_server.send(notification)?;
      }
      Request::GetPlainGoals(sender) => response
        .to_value_from_value::<GetPlainGoalsResponse>()?
        .send_to_oneshot(sender)?,
      Request::Hover(sender) => response
        .to_value_from_value::<HoverFileResponse>()?
        .send_to_oneshot(sender)?,

      // explicitly name ignored requests so new variants cause a compile error.
      Request::TextDocumentDocumentSymbol
      | Request::TextDocumentDocumentCodeAction
      | Request::TextDocumentFoldingRange
      | Request::LeanRpcConnect => (),
    }

    ().ok()
  }

  #[allow(clippy::unused_self)]
  #[tracing::instrument(skip_all)]
  fn process_request(&self, request: &Json) {
    tracing::info!(received_request = request.to_value(), "received request");
  }

  #[tracing::instrument(skip_all)]
  fn process_notification(&mut self, notification: Json) {
    tracing::info!(received_notification = notification.to_value(), "received notification");

    self.notifications.send(notification).log_if_error().unit()
  }

  #[tracing::instrument(skip_all, err)]
  fn process_message(&mut self, message: Json) -> Result<(), AnyhowError> {
    tracing::info!(received_message = message.to_value(), "received message");

    let Some(id) = message.get("id") else { return self.process_notification(message).ok() };
    let id = id.to_value_from_value::<Id>()?;

    if let Some(request) = self.requests.remove(&id) {
      self.process_response(request, &message)
    } else {
      self.process_request(&message).ok()
    }
  }

  #[tracing::instrument(skip_all)]
  async fn result(mut self) -> Result<(), AnyhowError> {
    loop {
      tokio::select! {
        session_command_res = self.commands.next_item_async() => self.process_command(session_command_res?).await?,
        json_message_res = self.lean_server.recv::<Json>() => self.process_message(json_message_res?)?,
      }
    }
  }

  pub async fn run(self) -> SessionResult {
    let id = self.id;
    let result = self.result().await;

    SessionResult { id, result }
  }
}
