use anyhow::Error;
use reqwest::Client as ReqwestClient;

use crate::{
  server::{GetSessionsResult, Server},
  utils::Utils,
};

pub struct Client {
  http: ReqwestClient,
  port: u16,
}

impl Client {
  pub fn new(port: u16) -> Result<Self, Error> {
    let http = Self::http()?;
    let client = Self { http, port };

    client.ok()
  }

  fn http() -> Result<ReqwestClient, Error> {
    ReqwestClient::builder().http2_prior_knowledge().build()?.ok()
  }

  fn url(&self, path: &str) -> String {
    std::format!("http://{url}:{port}{path}", url = Server::IPV4_ADDR, port = self.port)
  }

  pub async fn get(&self) -> Result<GetSessionsResult, Error> {
    let url = self.url(Server::GET_SESSIONS_PATH);

    self
      .http
      .get(url)
      .send()
      .await?
      .check_status()
      .await?
      .json::<GetSessionsResult>()
      .await?
      .ok()
  }
}
