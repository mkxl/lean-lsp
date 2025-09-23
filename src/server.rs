use std::net::Ipv4Addr;

use anyhow::Error;
use derive_more::From;
use poem::{
  Endpoint, EndpointExt, Error as PoemError, Route, Server as PoemServer, listener::TcpListener, middleware::Tracing,
};
use poem_openapi::{Object, OpenApi, OpenApiService, payload::Json};
use serde::Deserialize;
use ulid::Ulid;

use crate::{
  session::SessionClient,
  session_set::{NewSessionCommand, SessionSet, SessionSetClient},
  utils::Utils,
};

#[derive(From, Object)]
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
  pub const GET_SESSIONS_PATH: &'static str = "/session";
  pub const DEFAULT_PORT: u16 = 8080;
  pub const IPV4_ADDR: Ipv4Addr = Ipv4Addr::UNSPECIFIED;

  const ROOT_PATH: &'static str = "/";
  const OPEN_API_PATH: &'static str = "/openapi";
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
  async fn get_sessions(&self) -> Result<Json<GetSessionsResult>, PoemError> {
    self
      .session_set_client
      .get_sessions()
      .await?
      .iter()
      .map(SessionClient::id)
      .collect::<Vec<Ulid>>()
      .convert::<GetSessionsResult>()
      .poem_json()
      .ok()
  }

  #[oai(path = "/session/new", method = "post")]
  async fn new_session(&self, Json(command): Json<NewSessionCommand>) -> Result<Json<NewSessionResult>, PoemError> {
    self
      .session_set_client
      .new_session(command.lean_path.into(), command.lean_server_log_dirpath.map_into())
      .await?
      .id()
      .convert::<NewSessionResult>()
      .poem_json()
      .ok()
  }

  fn open_api_endpoint(open_api_service: &OpenApiService<Self, ()>) -> impl Endpoint<Output = String> + use<> {
    let spec_yaml = open_api_service.spec_yaml();
    let func = move |_request| spec_yaml.clone();

    poem::endpoint::make_sync(func)
  }

  pub async fn serve(port: u16) -> Result<(), Error> {
    let listener = TcpListener::bind((Self::IPV4_ADDR, port));
    let open_api_service = OpenApiService::new(Self::new(), Self::TITLE, Self::VERSION);
    let open_api_endpoint = Self::open_api_endpoint(&open_api_service);
    let endpoint = Route::new()
      .nest(Self::ROOT_PATH, open_api_service)
      .nest(Self::OPEN_API_PATH, open_api_endpoint)
      .with(Tracing);

    PoemServer::new(listener).run(endpoint).await?;

    ().ok()
  }
}
