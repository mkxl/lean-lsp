use std::{net::Ipv4Addr, path::PathBuf};

use anyhow::Error as AnyhowError;
use derive_more::From;
use mkutils::Utils;
use poem::{EndpointExt, Error as PoemError, Route, Server as PoemServer, listener::TcpListener, middleware::Tracing};
use poem_openapi::{Object, OpenApi, OpenApiService, param::Query, payload::Json as PoemJson};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use ulid::Ulid;

use crate::{
  commands::{NewSessionCommand, OpenFileCommand},
  session::Session,
  session_set::SessionSet,
  types::{GetPlainGoalsResult, Location, SessionSetStatus, SessionStatus},
};

#[derive(From, Deserialize, Object, Serialize)]
pub struct NewSessionResult {
  pub session_id: Ulid,
}

#[derive(Deserialize, From, Object)]
pub struct GetSessionsResult {
  pub sessions: Vec<SessionStatus>,
}

#[derive(Deserialize, From, Object, Serialize)]
pub struct GetNotificationsResult {
  pub notifications: Vec<Json>,
}

#[derive(Default)]
pub struct Server {
  session_set: SessionSet,
}

#[OpenApi]
impl Server {
  pub const DEFAULT_PORT: u16 = 8080;
  pub const IPV4_ADDR: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
  pub const PATH_GET_NOTIFICATIONS: &'static str = "/session/notifications";
  pub const PATH_GET_PLAIN_GOALS: &'static str = "/session/info-view/plain-goals";
  pub const PATH_GET_SESSIONS: &'static str = "/session";
  pub const PATH_GET_STATUS: &'static str = "/status";
  pub const PATH_NEW_SESSION: &'static str = "/session/new";
  pub const PATH_OPEN_FILE: &'static str = "/session/open";
  pub const QUERY_PARAM_CHARACTER: &'static str = "character";
  pub const QUERY_PARAM_FILEPATH: &'static str = "filepath";
  pub const QUERY_PARAM_LINE: &'static str = "line";
  pub const QUERY_PARAM_SESSION_ID: &'static str = "session_id";

  const PATH_OPEN_API: &'static str = "/openapi";
  const PATH_ROOT: &'static str = "/";
  const TITLE: &'static str = std::env!("CARGO_PKG_NAME");
  const VERSION: &'static str = std::env!("CARGO_PKG_VERSION");

  #[oai(path = "/status", method = "get")]
  #[allow(clippy::unused_async)]
  async fn get(&self) -> Result<PoemJson<SessionSetStatus>, PoemError> {
    self.session_set.status().await?.poem_json().ok()
  }

  #[oai(path = "/session", method = "get")]
  async fn get_sessions(
    &self,
    Query(session_id): Query<Option<Ulid>>,
  ) -> Result<PoemJson<GetSessionsResult>, PoemError> {
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
      .convert::<GetSessionsResult>()
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/new", method = "post")]
  async fn new_session(
    &self,
    PoemJson(command): PoemJson<NewSessionCommand>,
  ) -> Result<PoemJson<NewSessionResult>, PoemError> {
    let session = self
      .session_set
      .new_session(command.lean_path, command.lean_server_log_dirpath)
      .await?;

    session.initialize().await?;

    session.id().convert::<NewSessionResult>().poem_json().ok()
  }

  #[oai(path = "/session/open", method = "post")]
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

  #[oai(path = "/session/notifications", method = "get")]
  async fn notifications(
    &self,
    Query(session_id): Query<Option<Ulid>>,
  ) -> Result<PoemJson<GetNotificationsResult>, PoemError> {
    self
      .session_set
      .get_session(session_id)
      .await?
      .notifications()
      .await?
      .convert::<GetNotificationsResult>()
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/info-view/plain-goals", method = "get")]
  async fn get_plain_goals(
    &self,
    Query(session_id): Query<Option<Ulid>>,
    Query(filepath): Query<PathBuf>,
    Query(line): Query<usize>,
    Query(character): Query<usize>,
  ) -> Result<PoemJson<GetPlainGoalsResult>, PoemError> {
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

  pub async fn serve(port: u16) -> Result<(), AnyhowError> {
    let listener = TcpListener::bind((Self::IPV4_ADDR, port));
    let open_api_service = OpenApiService::new(Self::default(), Self::TITLE, Self::VERSION);
    let open_api_endpoint = open_api_service.spec_yaml().into_endpoint();
    let endpoint = Route::new()
      .nest(Self::PATH_ROOT, open_api_service)
      .nest(Self::PATH_OPEN_API, open_api_endpoint)
      .with(Tracing);

    PoemServer::new(listener).run(endpoint).await?;

    ().ok()
  }
}
