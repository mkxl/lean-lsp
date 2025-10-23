use anyhow::Error as AnyhowError;
use mkutils::Utils;
use reqwest::Client as ReqwestClient;
use ulid::Ulid;

use crate::{
  commands::{GetPlainGoalsCommand, NewSessionCommand, OpenFileCommand},
  server::{GetPlainGoalsResult, GetSessionsResult, NewSessionResult, Server},
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

  pub async fn new_session(&self, command: &NewSessionCommand) -> Result<NewSessionResult, AnyhowError> {
    let url = self.url(Server::PATH_NEW_SESSION);

    self
      .http_client
      .post(url)
      .json(command)
      .send()
      .await?
      .check_status()
      .await?
      .json::<NewSessionResult>()
      .await?
      .ok()
  }

  pub async fn open_file(&self, command: &OpenFileCommand) -> Result<(), AnyhowError> {
    let url = self.url(Server::PATH_OPEN_FILE);

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

  pub async fn get(&self, session_id: Option<Ulid>) -> Result<GetSessionsResult, AnyhowError> {
    let url = self.url(Server::PATH_GET_SESSIONS);

    self
      .http_client
      .get(url)
      .query_once(Server::QUERY_PARAM_SESSION_ID, session_id)
      .send()
      .await?
      .check_status()
      .await?
      .json::<GetSessionsResult>()
      .await?
      .ok()
  }

  pub async fn get_plain_goals(
    &self,
    session_id: Option<Ulid>,
    command: GetPlainGoalsCommand,
  ) -> Result<GetPlainGoalsResult, AnyhowError> {
    let url = self.url(Server::PATH_GET_PLAIN_GOALS);

    self
      .http_client
      .get(url)
      .query_once(Server::QUERY_PARAM_SESSION_ID, session_id)
      .query_once(Server::QUERY_PARAM_FILEPATH, command.filepath.some())
      .query_once(Server::QUERY_PARAM_LINE, command.line.some())
      .query_once(Server::QUERY_PARAM_CHARACTER, command.character.some())
      .send()
      .await?
      .check_status()
      .await?
      .json::<GetPlainGoalsResult>()
      .await?
      .ok()
  }
}
