use std::{collections::HashMap, io::Error as IoError};

use anyhow::Error as AnyhowError;
use camino::{Utf8Path, Utf8PathBuf};
use derive_more::{Constructor, Debug as DeriveMoreDebug};
use futures::{SinkExt, StreamExt};
use mkutils::{ProcessBuilder, Socket, Utils};
use tokio::{sync::broadcast::Sender as BroadcastSender, task::JoinSet};
use tokio_stream::wrappers::BroadcastStream as BroadcastReceiverStream;
use ulid::Ulid;

use crate::{
  commands::{
    ChangeFileCommand, CloseFileCommand, GetPlainGoalsCommand, HoverFileCommand, NewSessionCommand,
    NotificationsCommand, OpenFileCommand,
  },
  lean_server_process::LeanServerProcess,
  message::{Id, Message},
  notification::Notification,
  open_file::{OpenFile, OpenFileMap},
  responses::{GetPlainGoalsResponse, HoverFileResponse},
  types::{AppError, Position, RpcConnected, SessionInfo, Utf16},
};

#[derive(Constructor)]
pub struct SessionMessage<'a> {
  pub session: &'a mut Session,
  pub message: Result<Message, AnyhowError>,
}

#[derive(DeriveMoreDebug)]
enum Request {
  GetPlainGoals(#[debug(skip)] Socket),
  HoverFile(#[debug(skip)] Socket),
  Initialize(#[debug(skip)] Socket),
  LeanRpcConnect,
  TextDocumentDocumentCodeAction,
  TextDocumentDocumentSymbol,
  TextDocumentFoldingRange,
}

pub struct Session {
  id: Ulid,
  lake_session_id: Option<String>,
  project_absolute_dirpath: Utf8PathBuf,
  open_files: OpenFileMap,
  lean_server_process: LeanServerProcess,
  new_session_command: NewSessionCommand,
  requests: HashMap<Id, Request>,
  notifications: BroadcastSender<Notification>,
  join_set: JoinSet<Result<(), AnyhowError>>,
}

impl Session {
  pub const DEFAULT_PATH_STR: &str = ".";

  const LAKE_BUILD_SUBCOMMAD: &str = "build";
  const MANIFEST_FILE_NAME: &str = "lake-manifest.json";
  const MISSING_MANIFEST_ERROR_MESSAGE: &str =
    "unable to get project dirpath: no manifest file found in ancestor directories";
  const NOTIFICATIONS_CAPACITY: usize = 32;
  const SEND_NOTIFICATION_CONTEXT: &str = "unable to send notification";

  pub fn new(new_session_command: NewSessionCommand) -> Result<Self, AppError> {
    let id = Ulid::new();
    let lake_session_id = None;
    let open_files = OpenFileMap::default();
    let project_absolute_dirpath = Self::project_absolute_dirpath(&new_session_command.absolute_path)?;
    let lean_server_process = LeanServerProcess::new(
      &new_session_command.lake_filepath,
      new_session_command.lean_server_log_dirpath.as_deref(),
      &project_absolute_dirpath,
    )?;
    let requests = HashMap::new();
    let (notifications, _notifications_receiver) = tokio::sync::broadcast::channel(Self::NOTIFICATIONS_CAPACITY);
    let join_set = JoinSet::new();
    let session = Self {
      id,
      lake_session_id,
      project_absolute_dirpath,
      open_files,
      lean_server_process,
      new_session_command,
      requests,
      notifications,
      join_set,
    };

    session.ok()
  }

  fn project_absolute_dirpath(absolute_path: &Utf8Path) -> Result<Utf8PathBuf, AnyhowError> {
    for ancestor_path in absolute_path.ancestors() {
      let mut manifest_filepath = ancestor_path.join(Self::MANIFEST_FILE_NAME);

      if manifest_filepath.is_file() {
        manifest_filepath.pop();

        return manifest_filepath.ok();
      }
    }

    anyhow::bail!(Self::MISSING_MANIFEST_ERROR_MESSAGE);
  }

  pub const fn id(&self) -> Ulid {
    self.id
  }

  pub fn info(&self) -> SessionInfo {
    SessionInfo::new(self.id, self.project_absolute_dirpath.clone())
  }

  pub async fn next_message(&mut self) -> SessionMessage<'_> {
    let message = self.lean_server_process.next_message().await;

    SessionMessage::new(self, message)
  }

  fn on_notification(&self, message: Message) -> Result<(), AnyhowError> {
    tracing::info!(
      received_notification = message.json.as_valuable(),
      "received notification"
    );

    let notification = message.json.into_value_from_json()?;

    if let Err(send_error) = self.notifications.send(notification) {
      send_error
        .anyhow_msg_error()
        .context(Self::SEND_NOTIFICATION_CONTEXT)
        .log_error();
    }

    ().ok()
  }

  fn on_get_plain_goals_response(message: Message) -> Result<GetPlainGoalsResponse, AppError> {
    message.json.into_value_from_json::<GetPlainGoalsResponse>()?.ok()
  }

  fn on_hover_file_response(message: Message) -> Result<HoverFileResponse, AppError> {
    message.json.into_value_from_json::<HoverFileResponse>()?.ok()
  }

  async fn on_initialize_response(&mut self) -> Result<SessionInfo, AppError> {
    self.send_notification(Message::initialized_notification()).await?;

    self.info().ok()
  }

  fn on_lean_rpc_connect_response(&mut self, message: Message) -> Result<(), AnyhowError> {
    if self.lake_session_id.is_some() {
      return ().ok();
    }

    self.lake_session_id = message.into_response::<RpcConnected>()?.session_id.some();

    ().ok()
  }

  async fn on_response(&mut self, request: Request, message: Message) -> Result<(), AnyhowError> {
    tracing::info!(
      ?request,
      received_response = message.json.as_valuable(),
      "received response"
    );

    match request {
      Request::GetPlainGoals(socket) => {
        Self::on_get_plain_goals_response(message)
          .respond_to::<GetPlainGoalsCommand>(socket)
          .await?;
      }
      Request::HoverFile(socket) => {
        Self::on_hover_file_response(message)
          .respond_to::<HoverFileCommand>(socket)
          .await?;
      }
      Request::Initialize(socket) => {
        self
          .on_initialize_response()
          .await
          .respond_to::<NewSessionCommand>(socket)
          .await?;
      }
      Request::LeanRpcConnect => self.on_lean_rpc_connect_response(message)?,
      Request::TextDocumentDocumentCodeAction
      | Request::TextDocumentDocumentSymbol
      | Request::TextDocumentFoldingRange => (),
    }
    .ok()
  }

  fn on_request(message: &Message) {
    tracing::info!(received_request = message.json.as_valuable(), "received request");
  }

  pub async fn on_message(&mut self, message: Result<Message, AnyhowError>) -> Result<(), AnyhowError> {
    let mut message = message?;

    if self.new_session_command.enrich_utf16_positions {
      self.open_files.enrich_positions(&mut message.json);
    }

    let Some(id) = &message.id else { return self.on_notification(message) };

    if let Some(request) = self.requests.remove(id) {
      self.on_response(request, message).await
    } else {
      Self::on_request(&message).ok()
    }
  }

  async fn send(&mut self, message: Message) -> Result<(), AnyhowError> {
    self.lean_server_process.send(message).await
  }

  async fn send_request(&mut self, request: Request, message: Message) -> Result<(), AnyhowError> {
    let Some(id) = &message.id else { anyhow::bail!("unable to send request with no id") };

    if self.requests.insert(id.clone(), request).is_some() {
      tracing::warn!(%id, "registering request with existing id");
    }

    self.send(message).await
  }

  async fn send_notification(&mut self, message: Message) -> Result<(), AnyhowError> {
    self.send(message).await
  }

  pub async fn initialize(&mut self, socket: Socket) -> Result<(), AnyhowError> {
    let root_uri = self.project_absolute_dirpath.to_uri()?;
    let name = self.project_absolute_dirpath.file_name_ok()?;
    let request = Request::Initialize(socket);
    let message = Message::initialize_request(&self.project_absolute_dirpath, &root_uri, name);

    self.send_request(request, message).await
  }

  pub async fn send_keep_alive(&mut self) -> Result<(), AnyhowError> {
    let Some(lake_session_id) = self.lake_session_id.as_deref() else { return ().ok() };
    let root_uri = self.project_absolute_dirpath.to_uri()?;
    let message = Message::lean_rpc_keep_alive_notification(&root_uri, lake_session_id);

    self.send_notification(message).await
  }

  pub async fn change_file(&mut self, change_file_command: &ChangeFileCommand) -> Result<(), AppError> {
    let open_file = self.open_files.get_mut(&change_file_command.filepath)?;
    let new_version = open_file.increment_version();
    let uri = change_file_command.filepath.to_uri()?;
    let text = change_file_command
      .input_filepath
      .as_path()
      .read_to_string_fs_async()
      .await
      .result?;
    let message = Message::text_document_did_change_notification(&text, &uri, new_version);

    self.send_notification(message).await?;

    ().ok()
  }

  pub async fn close_file(&mut self, close_file_command: &CloseFileCommand) -> Result<(), AppError> {
    self.open_files.check_contains(&close_file_command.filepath)?;

    let uri = close_file_command.filepath.to_uri()?;
    let message = Message::text_document_did_close_notification(&uri);

    self.send_notification(message).await?;
    self.open_files.remove(&close_file_command.filepath);

    ().ok()
  }

  pub async fn hover_file(&mut self, socket: Socket, hover_file_command: &HoverFileCommand) -> Result<(), AnyhowError> {
    let open_file = self.open_files.get(&hover_file_command.location.filepath)?;
    let position = Position::<Utf16>::from_utf8(hover_file_command.location.position, open_file.text())?;
    let uri = hover_file_command.location.filepath.to_uri()?;
    let request = Request::HoverFile(socket);
    let message = Message::text_document_hover_request(&uri, position);

    self.send_request(request, message).await?;

    ().ok()
  }

  pub async fn open_file(&mut self, open_file_command: OpenFileCommand) -> Result<(), AppError> {
    self.open_files.check_doesnt_contain(&open_file_command.filepath)?;

    let uri = open_file_command.filepath.to_uri()?;
    let text = open_file_command
      .filepath
      .as_path()
      .read_to_string_fs_async()
      .await
      .result?;
    let open_file = OpenFile::new(text);
    let text_document_did_open_notification_message =
      Message::text_document_did_open_notification(open_file.text(), &uri);
    let text_document_document_symbol_request_message = Message::text_document_document_symbol_request(&uri);
    let text_document_document_code_action_request_message = Message::text_document_document_code_action_request(&uri);
    let text_document_folding_range_request_message = Message::text_document_folding_range_request(&uri);
    let lean_rpc_connect_request_message = Message::lean_rpc_connect_request(&uri);

    self
      .send_notification(text_document_did_open_notification_message)
      .await?;
    self
      .send_request(
        Request::TextDocumentDocumentSymbol,
        text_document_document_symbol_request_message,
      )
      .await?;
    self
      .send_request(
        Request::TextDocumentDocumentCodeAction,
        text_document_document_code_action_request_message,
      )
      .await?;
    self
      .send_request(
        Request::TextDocumentFoldingRange,
        text_document_folding_range_request_message,
      )
      .await?;
    self
      .send_request(Request::LeanRpcConnect, lean_rpc_connect_request_message)
      .await?;

    self.open_files.insert(open_file_command.filepath, open_file);

    ().ok()
  }

  pub async fn get_plain_goals(
    &mut self,
    socket: Socket,
    get_plain_goals_command: &GetPlainGoalsCommand,
  ) -> Result<(), AnyhowError> {
    let open_file = self.open_files.get(&get_plain_goals_command.location.filepath)?;
    let position = Position::<Utf16>::from_utf8(get_plain_goals_command.location.position, open_file.text())?;
    let uri = get_plain_goals_command.location.filepath.to_uri()?;
    let request = Request::GetPlainGoals(socket);
    let message = Message::lean_rpc_get_plain_goals_request(&uri, position);

    self.send_request(request, message).await?;

    ().ok()
  }

  pub async fn kill(&mut self) -> Result<(), IoError> {
    self.lean_server_process.kill().await?;
    self.join_set.abort_all();

    ().ok()
  }

  pub fn notification_stream(&self) -> BroadcastReceiverStream<Notification> {
    self.notifications.subscribe().into_stream()
  }

  #[tracing::instrument(skip_all, err)]
  async fn notify_impl(
    mut socket: Socket,
    notifications_command: NotificationsCommand,
    mut notification_stream: BroadcastReceiverStream<Notification>,
  ) -> Result<(), AnyhowError> {
    while let Some(notification_res) = notification_stream.next().await {
      let notification = notification_res?;

      if notifications_command.methods.is_empty() || notifications_command.methods.contains_eq(notification.method()) {
        socket.send(notification).await?;
      }
    }

    ().ok()
  }

  pub fn notify(&mut self, socket: Socket, notifications_command: NotificationsCommand) {
    let notify_future = Self::notify_impl(socket, notifications_command, self.notification_stream());

    self.join_set.spawn(notify_future);
  }

  fn reset(&mut self) -> Result<(), AnyhowError> {
    self.lake_session_id.take();
    self.open_files.clear();
    self.requests.clear();

    self.lean_server_process = LeanServerProcess::new(
      &self.new_session_command.lake_filepath,
      self.new_session_command.lean_server_log_dirpath.as_deref(),
      &self.project_absolute_dirpath,
    )?;

    ().ok()
  }

  pub async fn rebuild(&mut self) -> Result<(), AppError> {
    let rebuild_output = ProcessBuilder::new(&self.new_session_command.lake_filepath)
      .arg(Self::LAKE_BUILD_SUBCOMMAD)
      .current_dirpath(&self.project_absolute_dirpath)
      .command_mut()
      .spawn()?
      .wait_with_output()
      .await?;

    tracing::info!(?rebuild_output);

    rebuild_output.exit_ok()?;
    self.reset()?;

    ().ok()
  }
}
