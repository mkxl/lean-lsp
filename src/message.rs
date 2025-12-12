pub mod initialize;
pub mod lean_rpc;
pub mod text_document;

use camino::Utf8Path;
use derive_more::{Constructor, Display, From};
use mkutils::Utils;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use ulid::Ulid;

use crate::types::{Position, Utf16};

// NOTE:
// - order of variants is significant because the enum is untagged (serde will
//   attempt to deserialize a ulid first, etc)
// - derive [Eq, Hash, PartialEq] so [Id] can be used as a hash map key
#[derive(Clone, Deserialize, Display, Eq, From, Hash, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Id {
  Ulid(Ulid),
  Usize(usize),
  Named(String),
}

#[derive(Constructor, Deserialize, Serialize)]
pub struct Message {
  pub id: Option<Id>,

  #[serde(flatten)]
  pub json: Json,
}

impl Message {
  fn json(method: &str, params: &Json) -> Json {
    serde_json::json!({
      "jsonrpc": "2.0",
      "method": method,
      "params": params,
    })
  }

  fn request(method: &str, params: &Json) -> Self {
    let json = Self::json(method, params);

    Self::new(Ulid::new().convert::<Id>().some(), json)
  }

  fn notification(method: &str, params: &Json) -> Self {
    let json = Self::json(method, params);

    Self::new(None, json)
  }

  pub fn initialize_request(root_path: &Utf8Path, root_uri: &str, name: &str) -> Self {
    let params = crate::message::initialize::initialize_params(root_path, root_uri, name, std::process::id());

    Self::request("initialize", &params)
  }

  pub fn initialized_notification() -> Self {
    let params = crate::message::initialize::initialized_params();

    Self::notification("initialized", &params)
  }

  pub fn lean_rpc_connect_request(uri: &str) -> Self {
    let params = crate::message::lean_rpc::connect_params(uri);

    Self::request("$/lean/rpc/connect", &params)
  }

  pub fn lean_rpc_get_plain_goals_request(uri: &str, position: Position<Utf16>) -> Self {
    let params = crate::message::lean_rpc::get_plain_goals_params(uri, position);

    Self::request("$/lean/plainGoal", &params)
  }

  pub fn text_document_did_change_notification(text: &str, uri: &str, version: usize) -> Self {
    let params = crate::message::text_document::did_change_notification_params(text, uri, version);

    Self::notification("textDocument/didChange", &params)
  }

  pub fn text_document_did_close_notification(uri: &str) -> Self {
    let params = crate::message::text_document::did_close_notification_params(uri);

    Self::notification("textDocument/didClose", &params)
  }

  pub fn text_document_did_open_notification(text: &str, uri: &str) -> Self {
    let params = crate::message::text_document::did_open_notification_params(text, uri);

    Self::notification("textDocument/didOpen", &params)
  }

  pub fn text_document_document_code_action_request(uri: &str) -> Self {
    let params = crate::message::text_document::document_code_action_params(uri);

    Self::request("textDocument/codeAction", &params)
  }

  pub fn text_document_hover_request(uri: &str, position: Position<Utf16>) -> Self {
    let params = crate::message::text_document::hover_params(uri, position);

    Self::request("textDocument/hover", &params)
  }

  pub fn text_document_document_symbol_request(uri: &str) -> Self {
    let params = crate::message::text_document::document_symbol_params(uri);

    Self::request("textDocument/documentSymbol", &params)
  }

  pub fn text_document_folding_range_request(uri: &str) -> Self {
    let params = crate::message::text_document::folding_range_params(uri);

    Self::request("textDocument/foldingRange", &params)
  }
}
