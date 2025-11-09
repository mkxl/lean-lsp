use anyhow::Error as AnyhowError;
use futures::{Stream, StreamExt};
use mkutils::Utils;
use reqwest::Client as ReqwestClient;
use serde::Serialize;
use serde_json::Value as Json;
use ulid::Ulid;

use crate::{
  commands::{ChangeFileCommand, CloseFileCommand, NewSessionCommand, OpenFileCommand},
  server::{
    Server,
    requests::ChangeFileRequest,
    responses::{GetPlainGoalsResponse, GetSessionsResponse, NewSessionResponse},
  },
  types::{Location, SessionSetStatus},
};

pub struct Client {
  http_client: ReqwestClient,
  port: u16,
}

impl Client {
  pub fn new(port: u16) -> Result<Self, AnyhowError> {
    let http_client = Self::http_client()?;
    let client = Self { http_client, port };

    client.ok()
  }

  fn http_client() -> Result<ReqwestClient, AnyhowError> {
    ReqwestClient::builder().http2_prior_knowledge().build()?.ok()
  }

  fn url(&self, path: &str) -> String {
    std::format!("http://{url}:{port}{path}", url = Server::IPV4_ADDR, port = self.port)
  }

  pub async fn new_session(&self, command: &NewSessionCommand) -> Result<NewSessionResponse, AnyhowError> {
    let url = self.url(Server::PATH_NEW_SESSION);

    self
      .http_client
      .post(url)
      .json(command)
      .send()
      .await?
      .check_status()
      .await?
      .json::<NewSessionResponse>()
      .await?
      .ok()
  }

  pub async fn open_file(&self, command: &OpenFileCommand) -> Result<(), AnyhowError> {
    let url = self.url(Server::PATH_FILE_OPEN);

    self
      .http_client
      .post(url)
      .json(command)
      .send()
      .await?
      .check_status()
      .await?
      .json::<()>()
      .await?
      .ok()
  }

  pub async fn change_file(&self, command: ChangeFileCommand) -> Result<(), AnyhowError> {
    let url = self.url(Server::PATH_FILE_CHANGE);
    let request = ChangeFileRequest::new(command).await?;

    self
      .http_client
      .post(url)
      .json(&request)
      .send()
      .await?
      .check_status()
      .await?
      .json::<()>()
      .await?
      .ok()
  }

  pub async fn close_file(&self, command: &CloseFileCommand) -> Result<(), AnyhowError> {
    let url = self.url(Server::PATH_FILE_CLOSE);

    self
      .http_client
      .post(url)
      .json(command)
      .send()
      .await?
      .check_status()
      .await?
      .json::<()>()
      .await?
      .ok()
  }

  pub async fn get(&self, session_id: Option<Ulid>) -> Result<GetSessionsResponse, AnyhowError> {
    let url = self.url(Server::PATH_GET_SESSIONS);

    self
      .http_client
      .get(url)
      .query_one::<Ulid>(Server::QUERY_PARAM_SESSION_ID, session_id)
      .send()
      .await?
      .check_status()
      .await?
      .json::<GetSessionsResponse>()
      .await?
      .ok()
  }

  pub async fn notifications<T: Serialize>(
    &self,
    session_id: Option<Ulid>,
    methods: &[T],
  ) -> Result<impl Stream<Item = Result<Json, AnyhowError>>, AnyhowError> {
    let url = self.url(Server::PATH_GET_NOTIFICATIONS);

    self
      .http_client
      .get(url)
      .query_one::<Ulid>(Server::QUERY_PARAM_SESSION_ID, session_id)
      .query_all(Server::QUERY_PARAM_METHODS, methods)
      .send()
      .await?
      .check_status()
      .await?
      .bytes_stream()
      .map(Utils::io_result)
      .into_stream_reader()
      .into_line_frames()
      .map(|line_res| line_res?.to_value_from_json_byte_str::<Json>()?.ok())
      .ok()
  }

  pub async fn get_plain_goals(
    &self,
    session_id: Option<Ulid>,
    location: Location,
  ) -> Result<GetPlainGoalsResponse, AnyhowError> {
    let url = self.url(Server::PATH_GET_PLAIN_GOALS);

    self
      .http_client
      .get(url)
      .query_one::<Ulid>(Server::QUERY_PARAM_SESSION_ID, session_id)
      .query_one(Server::QUERY_PARAM_FILEPATH, location.filepath)
      .query_one(Server::QUERY_PARAM_LINE, location.line)
      .query_one(Server::QUERY_PARAM_CHARACTER, location.character)
      .send()
      .await?
      .check_status()
      .await?
      .json::<GetPlainGoalsResponse>()
      .await?
      .ok()
  }

  pub async fn status(&self) -> Result<SessionSetStatus, AnyhowError> {
    let url = self.url(Server::PATH_GET_STATUS);

    self
      .http_client
      .get(url)
      .send()
      .await?
      .check_status()
      .await?
      .json::<SessionSetStatus>()
      .await?
      .ok()
  }

  pub async fn kill(&self, session_id: Option<Ulid>) -> Result<(), AnyhowError> {
    let url = self.url(Server::PATH_KILL_SESSION);

    self
      .http_client
      .delete(url)
      .query_one::<Ulid>(Server::QUERY_PARAM_SESSION_ID, session_id)
      .send()
      .await?
      .check_status()
      .await?
      .json::<()>()
      .await?
      .ok()
  }
}
