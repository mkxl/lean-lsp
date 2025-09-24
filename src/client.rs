use anyhow::Error;
use mkutils::Utils;
use reqwest::Client as ReqwestClient;

use crate::server::{GetSessionsResult, Server};

pub struct Client {
  http_client: ReqwestClient,
  port: u16,
}

impl Client {
  pub fn new(port: u16) -> Result<Self, Error> {
    let http_client = Self::http_client()?;
    let client = Self { http_client, port };

    client.ok()
  }

  fn http_client() -> Result<ReqwestClient, Error> {
    ReqwestClient::builder().http2_prior_knowledge().build()?.ok()
  }

  fn url(&self, path: &str) -> String {
    std::format!("http://{url}:{port}{path}", url = Server::IPV4_ADDR, port = self.port)
  }

  pub async fn get(&self) -> Result<GetSessionsResult, Error> {
    let url = self.url(Server::GET_SESSIONS_PATH);

    self
      .http_client
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
