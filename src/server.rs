use std::net::Ipv4Addr;

use anyhow::Error;
use derive_more::From;
use mkutils::Utils;
use poem::{EndpointExt, Error as PoemError, Route, Server as PoemServer, listener::TcpListener, middleware::Tracing};
use poem_openapi::{Object, OpenApi, OpenApiService, param::Query, payload::Json};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{
  session::SessionClient,
  session_set::{NewSessionCommand, OpenFileCommand, SessionSet, SessionSetClient},
};

#[derive(From, Deserialize, Object, Serialize)]
pub struct NewSessionResult {
  pub session_id: Ulid,
}

#[derive(Deserialize, From, Object)]
pub struct GetSessionsResult {
  pub session_ids: Vec<Ulid>,
}

pub struct Server {
  session_set_client: SessionSetClient,
}

#[OpenApi]
impl Server {
  pub const PATH_GET_SESSIONS: &'static str = "/session";
  pub const PATH_NEW_SESSION: &'static str = "/session/new";
  pub const PATH_OPEN_FILE: &'static str = "/session/open";
  pub const DEFAULT_PORT: u16 = 8080;
  pub const IPV4_ADDR: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
  pub const SESSION_QUERY_PARAM_NAME: &'static str = "session_id";

  const PATH_ROOT: &'static str = "/";
  const PATH_OPEN_API: &'static str = "/openapi";
  const TITLE: &'static str = std::env!("CARGO_PKG_NAME");
  const VERSION: &'static str = std::env!("CARGO_PKG_VERSION");

  fn new() -> Self {
    let (session_set, session_set_client) = SessionSet::new();

    // NOTE:
    // - NOTE-97a211
    // - a dropped [JoinHandle] (the return type of [spawn_task()]) detaches from the associated task but leaves it
    //   still running: [https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html]
    session_set.run().spawn_task();

    Self { session_set_client }
  }

  #[oai(path = "/session", method = "get")]
  async fn get_sessions(&self, Query(session_id): Query<Option<Ulid>>) -> Result<Json<GetSessionsResult>, PoemError> {
    let session_clients = if session_id.is_some() {
      self
        .session_set_client
        .get_session_client(session_id)
        .await?
        .once()
        .collect()
    } else {
      self.session_set_client.get_session_clients().await?
    };

    session_clients
      .iter()
      .map(SessionClient::id)
      .collect::<Vec<Ulid>>()
      .log("session client ids")
      .convert::<GetSessionsResult>()
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/open", method = "post")]
  async fn open_file(&self, Json(command): Json<OpenFileCommand>) -> Result<Json<()>, PoemError> {
    self
      .session_set_client
      .get_session_client(command.session_id)
      .await?
      .open_file(command.lean_filepath)
      .await?
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/new", method = "post")]
  async fn new_session(&self, Json(command): Json<NewSessionCommand>) -> Result<Json<NewSessionResult>, PoemError> {
    self
      .session_set_client
      .new_session(command.lean_path, command.lean_server_log_dirpath)
      .await?
      .id()
      .convert::<NewSessionResult>()
      .poem_json()
      .ok()
  }

  pub async fn serve(port: u16) -> Result<(), Error> {
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
