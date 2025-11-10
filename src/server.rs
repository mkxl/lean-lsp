pub mod requests;
pub mod responses;

use std::{collections::HashSet, net::Ipv4Addr, path::PathBuf};

use anyhow::Error as AnyhowError;
use derive_more::From;
use futures::StreamExt;
use mkutils::Utils;
use poem::{
  Body as PoemBody, EndpointExt, Error as PoemError, Route, Server as PoemServer,
  listener::TcpListener,
  middleware::Tracing,
  web::websocket::{BoxWebSocketUpgraded, Message, WebSocket, WebSocketStream},
};
use poem_openapi::{
  OpenApi, OpenApiService,
  param::Query,
  payload::{Binary as PoemBinary, Json as PoemJson},
};
use tokio::task::JoinHandle;
use ulid::Ulid;

use crate::{
  commands::{CloseFileCommand, HoverFileCommand, NewSessionCommand, OpenFileCommand},
  server::{
    requests::ChangeFileRequest,
    responses::{GetPlainGoalsResponse, GetSessionsResponse, HoverFileResponse, NewSessionResponse},
  },
  session::Session,
  session_set::SessionSet,
  types::{Location, SessionSetStatus},
};

#[derive(From)]
pub struct Server {
  session_set: SessionSet,
  join_handle: JoinHandle<Result<(), AnyhowError>>,
}

#[OpenApi]
impl Server {
  pub const DEFAULT_PORT: u16 = 8080;
  pub const IPV4_ADDR: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
  pub const PATH_FILE_CHANGE: &'static str = "/session/file/change";
  pub const PATH_FILE_CLOSE: &'static str = "/session/file/close";
  pub const PATH_FILE_HOVER: &'static str = "/session/file/hover";
  pub const PATH_FILE_OPEN: &'static str = "/session/file/open";
  pub const PATH_GET_NOTIFICATIONS: &'static str = "/session/notifications";
  pub const PATH_GET_PLAIN_GOALS: &'static str = "/session/info-view/plain-goals";
  pub const PATH_GET_SESSIONS: &'static str = "/session";
  pub const PATH_GET_STATUS: &'static str = "/status";
  pub const PATH_KILL_SESSION: &'static str = "/session";
  pub const PATH_NEW_SESSION: &'static str = "/session/new";
  pub const QUERY_PARAM_CHARACTER: &'static str = "character";
  pub const QUERY_PARAM_FILEPATH: &'static str = "filepath";
  pub const QUERY_PARAM_LINE: &'static str = "line";
  pub const QUERY_PARAM_METHODS: &'static str = "methods";
  pub const QUERY_PARAM_SESSION_ID: &'static str = "session_id";

  const PATH_OPEN_API: &'static str = "/openapi";
  const PATH_ROOT: &'static str = "/";
  const TITLE: &'static str = std::env!("CARGO_PKG_NAME");
  const VERSION: &'static str = std::env!("CARGO_PKG_VERSION");

  fn new() -> Self {
    SessionSet::new().into()
  }

  async fn session_set_status(&self) -> Result<SessionSetStatus, AnyhowError> {
    let session_set = self.join_handle.is_finished().into();
    let sessions = self
      .session_set
      .get_sessions()
      .await?
      .iter()
      .map(Session::status)
      .try_join_all()
      .await?;
    let session_set_status = SessionSetStatus::new(session_set, sessions);

    session_set_status.ok()
  }

  #[oai(path = "/status", method = "get")]
  #[allow(clippy::unused_async)]
  async fn status(&self) -> Result<PoemJson<SessionSetStatus>, PoemError> {
    self.session_set_status().await?.poem_json().ok()
  }

  #[oai(path = "/session", method = "get")]
  async fn get_sessions(
    &self,
    Query(session_id): Query<Option<Ulid>>,
  ) -> Result<PoemJson<GetSessionsResponse>, PoemError> {
    let sessions = if session_id.is_some() {
      self.session_set.get_session(session_id).await?.once().collect()
    } else {
      self.session_set.get_sessions().await?
    };

    sessions
      .iter()
      .map(Session::status)
      .try_join_all()
      .await?
      .convert::<GetSessionsResponse>()
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/new", method = "post")]
  async fn new_session(
    &self,
    PoemJson(command): PoemJson<NewSessionCommand>,
  ) -> Result<PoemJson<NewSessionResponse>, PoemError> {
    let session = self
      .session_set
      .new_session(command.lean_path, command.lean_server_log_dirpath)
      .await?;

    session.initialize().await?;

    session.id().convert::<NewSessionResponse>().poem_json().ok()
  }

  #[oai(path = "/session/file/open", method = "post")]
  async fn open_file(&self, PoemJson(command): PoemJson<OpenFileCommand>) -> Result<PoemJson<()>, PoemError> {
    self
      .session_set
      .get_session(command.session_id)
      .await?
      .open_file(command.lean_filepath)
      .await?
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/file/change", method = "post")]
  async fn change_file(&self, PoemJson(command): PoemJson<ChangeFileRequest>) -> Result<PoemJson<()>, PoemError> {
    self
      .session_set
      .get_session(command.session_id)
      .await?
      .change_file(command.lean_filepath, command.text)
      .await?
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/file/close", method = "post")]
  async fn close_file(&self, PoemJson(command): PoemJson<CloseFileCommand>) -> Result<PoemJson<()>, PoemError> {
    self
      .session_set
      .get_session(command.session_id)
      .await?
      .close_file(command.lean_filepath)
      .await?
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/file/hover", method = "post")]
  async fn hover_file(
    &self,
    PoemJson(command): PoemJson<HoverFileCommand>,
  ) -> Result<PoemJson<HoverFileResponse>, PoemError> {
    self
      .session_set
      .get_session(command.session_id)
      .await?
      .hover_file(command.location)
      .await?
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/notifications", method = "get")]
  async fn notifications(
    &self,
    Query(session_id): Query<Option<Ulid>>,
    Query(methods): Query<HashSet<String>>,
  ) -> Result<PoemBinary<PoemBody>, PoemError> {
    self
      .session_set
      .get_session(session_id)
      .await?
      .notifications()
      .filter_sync(move |notification_json_res| {
        !mkutils::when! {
          !methods.is_empty()
            && let Ok(notification_json) = notification_json_res
            && let Some(method_json) = notification_json.get("method")
            && let Some(method) = method_json.as_str()
            && !methods.contains(method)
        }
      })
      .map(|notification_json_res| {
        notification_json_res?
          .to_json_byte_str()?
          .pushed(b'\n')
          .ok::<AnyhowError>()
      })
      .map(Utils::io_result)
      .poem_stream_body()
      .ok()
  }

  #[oai(path = "/session/info-view/plain-goals", method = "get")]
  async fn get_plain_goals(
    &self,
    Query(session_id): Query<Option<Ulid>>,
    Query(filepath): Query<PathBuf>,
    Query(line): Query<usize>,
    Query(character): Query<usize>,
  ) -> Result<PoemJson<GetPlainGoalsResponse>, PoemError> {
    let location = Location::new(filepath, line, character);
    let response = self
      .session_set
      .get_session(session_id)
      .await?
      .get_plain_goals(location)
      .await?
      .poem_json();

    response.ok()
  }

  async fn on_web_socket_upgrade(
    session_set: SessionSet,
    mut web_socket_stream: WebSocketStream,
  ) -> Result<(), AnyhowError> {
    loop {
      let Message::Text(message) = web_socket_stream.next_item_async().await?? else { continue };
      let mut message_json = message.to_json()?;
      let session_id = message_json.take_json("session_id")?;
      let response_json = match message_json.take_json::<String>("type")?.as_str() {
        "new_session" => session_set
          .new_session(
            message_json.take_json("lean_path")?,
            message_json.take_json("lean_server_log_dirpath")?,
          )
          .await?
          .id()
          .to_json_object("session_id"),
        "get_sessions" => session_set
          .get_sessions()
          .await?
          .iter()
          .map_collect::<Ulid, Vec<_>>(Session::id)
          .to_json_object("session_ids"),
        "get_session" => session_set
          .get_session(session_id)
          .await?
          .id()
          .to_json_object("session_id"),
        "initialize" => session_set
          .get_session(session_id)
          .await?
          .initialize()
          .await?
          .with("complete")
          .to_json_object("initialize"),
        "open_file" => session_set
          .get_session(session_id)
          .await?
          .open_file(message_json.take_json("filepath")?)
          .await?
          .with("complete")
          .to_json_object("open_file"),
        "close_file" => session_set
          .get_session(session_id)
          .await?
          .close_file(message_json.take_json("filepath")?)
          .await?
          .with("complete")
          .to_json_object("close_file"),
        "hover_file" => session_set
          .get_session(session_id)
          .await?
          .hover_file(message_json.take_json("location")?)
          .await?
          .to_json_object("hover_file"),
        "get_plain_goals" => session_set
          .get_session(session_id)
          .await?
          .get_plain_goals(message_json.take_json("location")?)
          .await?
          .to_json()?,
        "get_status" => session_set.get_session(session_id).await?.status().await?.to_json()?,
        _ => serde_json::json!({"error": "unknown type"}),
      };

      response_json
        .to_json_str()?
        .poem_text_message()
        .send_to(&mut web_socket_stream)
        .await?;
    }
  }

  #[allow(clippy::unused_async)]
  #[oai(path = "/stream", method = "get")]
  async fn stream(&self, web_socket: WebSocket) -> BoxWebSocketUpgraded {
    let session_set = self.session_set.clone();
    let web_socket_upgraded =
      web_socket.on_upgrade(|web_socket_stream| Self::on_web_socket_upgrade(session_set, web_socket_stream));

    web_socket_upgraded.boxed()
  }

  #[oai(path = "/session", method = "delete")]
  async fn kill_session(&self, Query(session_id): Query<Option<Ulid>>) -> Result<PoemJson<()>, PoemError> {
    self
      .session_set
      .get_session(session_id)
      .await?
      .kill()
      .await?
      .poem_json()
      .ok()
  }

  pub async fn serve(port: u16) -> Result<(), AnyhowError> {
    let listener = TcpListener::bind((Self::IPV4_ADDR, port));
    let open_api_service = OpenApiService::new(Self::new(), Self::TITLE, Self::VERSION);
    let open_api_endpoint = open_api_service.spec_yaml().into_endpoint();
    let endpoint = Route::new()
      .nest(Self::PATH_ROOT, open_api_service)
      .nest(Self::PATH_OPEN_API, open_api_endpoint)
      .with(Tracing);

    PoemServer::new(listener).run(endpoint).await?;

    ().ok()
  }
}
