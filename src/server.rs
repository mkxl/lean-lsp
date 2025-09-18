mod commands;

use std::net::Ipv4Addr;

use anyhow::Error;
use poem::{EndpointExt, Error as PoemError, Route, Server as PoemServer, listener::TcpListener, middleware::Tracing};
use poem_openapi::{OpenApi, OpenApiService, payload::Json};
use uuid::Uuid;

use crate::{
  server::commands::{GetSessionsResult, NewSessionCommand, NewSessionResult},
  session::SessionClient,
  session_set::{SessionSet, SessionSetClient},
  utils::Utils,
};

pub struct Server {
  session_set_client: SessionSetClient,
}

#[OpenApi]
impl Server {
  const IPV4_ADDR: Ipv4Addr = Ipv4Addr::UNSPECIFIED;
  const TITLE: &'static str = std::env!("CARGO_PKG_NAME");
  const VERSION: &'static str = std::env!("CARGO_PKG_VERSION");
  const ROOT_PATH: &'static str = "/";

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
      .collect::<Vec<Uuid>>()
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

  pub async fn serve(port: u16) -> Result<(), Error> {
    let listener = TcpListener::bind((Self::IPV4_ADDR, port));
    let open_api_service = OpenApiService::new(Self::new(), Self::TITLE, Self::VERSION);
    let endpoint = Route::new().nest(Self::ROOT_PATH, open_api_service).with(Tracing);

    PoemServer::new(listener).run(endpoint).await?;

    ().ok()
  }
}
