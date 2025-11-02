pub mod initialize;
pub mod lean_rpc;
pub mod text_document;

use std::path::Path;

use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use ulid::Ulid;

#[derive(Clone, Copy, Deserialize, Display, Eq, From, Hash, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Id {
  Ulid(Ulid),
  Usize(usize),
}

pub struct Message {
  pub id: Id,
  pub json: Json,
}

impl Message {
  fn request(method: &str, params: &Json) -> Self {
    let id = Ulid::new().into();
    let json = serde_json::json!({
      "jsonrpc": "2.0",
      "id": id,
      "method": method,
      "params": params,
    });

    Self { id, json }
  }

  fn notification(method: &str, params: &Json) -> Json {
    serde_json::json!({
      "jsonrpc": "2.0",
      "method": method,
      "params": params,
    })
  }

  pub fn initialize_request(root_path: &Path, root_uri: &str, name: &str) -> Self {
    let params = crate::messages::initialize::initialize_params(root_path, root_uri, name, std::process::id());

    Self::request("initialize", &params)
  }

  #[allow(clippy::unused_self)]
  pub fn initialized_notification() -> Json {
    let params = crate::messages::initialize::initialized_params();

    Self::notification("initialized", &params)
  }

  #[allow(clippy::unused_self)]
  pub fn text_document_did_open_notification(text: &str, uri: &str) -> Json {
    let params = crate::messages::text_document::did_open_notification_params(text, uri);

    Self::notification("textDocument/didOpen", &params)
  }

  pub fn text_document_document_symbol_request(uri: &str) -> Self {
    let params = crate::messages::text_document::document_symbol_params(uri);

    Self::request("textDocument/documentSymbol", &params)
  }

  pub fn text_document_document_code_action_request(uri: &str) -> Self {
    let params = crate::messages::text_document::document_code_action_params(uri);

    Self::request("textDocument/codeAction", &params)
  }

  pub fn text_document_folding_range_request(uri: &str) -> Self {
    let params = crate::messages::text_document::folding_range_params(uri);

    Self::request("textDocument/foldingRange", &params)
  }

  pub fn lean_rpc_connect_request(uri: &str) -> Self {
    let params = crate::messages::lean_rpc::connect_params(uri);

    Self::request("$/lean/rpc/connect", &params)
  }

  pub fn lean_rpc_get_plain_goals_request(uri: &str, line: usize, character: usize) -> Self {
    let params = crate::messages::lean_rpc::get_plain_goals_params(uri, line, character);

    Self::request("$/lean/plainGoal", &params)
  }
}
