pub mod initialize;
pub mod lean_rpc;
pub mod text_document;

use std::{path::Path, sync::atomic::AtomicUsize};

use derive_more::Constructor;
use mkutils::Utils;
use serde_json::Value as Json;

#[derive(Default)]
pub struct Messages {
  id: AtomicUsize,
}

#[derive(Constructor)]
pub struct RequestWithId {
  pub request: Json,
  pub id: usize,
}

impl Messages {
  fn request_with_id(&self, method: &str, params: &Json) -> RequestWithId {
    let id = self.id.inc();
    let request = serde_json::json!({
      "jsonrpc": "2.0",
      "id": id,
      "method": method,
      "params": params,
    });

    RequestWithId::new(request, id)
  }

  fn notification(method: &str, params: &Json) -> Json {
    serde_json::json!({
      "jsonrpc": "2.0",
      "method": method,
      "params": params,
    })
  }

  pub fn initialize_request(&self, root_path: &Path, root_uri: &str, name: &str) -> Json {
    let params = crate::messages::initialize::initialize_params(root_path, root_uri, name, std::process::id());

    self.request_with_id("initialize", &params).request
  }

  #[allow(clippy::unused_self)]
  pub fn initialized_notification(&self) -> Json {
    let params = crate::messages::initialize::initialized_params();

    Self::notification("initialized", &params)
  }

  #[allow(clippy::unused_self)]
  pub fn text_document_did_open_notification(&self, text: &str, uri: &str) -> Json {
    let params = crate::messages::text_document::did_open_notification_params(text, uri);

    Self::notification("textDocument/didOpen", &params)
  }

  pub fn text_document_document_symbol_request(&self, uri: &str) -> Json {
    let params = crate::messages::text_document::document_symbol_params(uri);

    self.request_with_id("textDocument/documentSymbol", &params).request
  }

  pub fn text_document_document_code_action_request(&self, uri: &str) -> Json {
    let params = crate::messages::text_document::document_code_action_params(uri);

    self.request_with_id("textDocument/codeAction", &params).request
  }

  pub fn text_document_folding_range_request(&self, uri: &str) -> Json {
    let params = crate::messages::text_document::folding_range_params(uri);

    self.request_with_id("textDocument/foldingRange", &params).request
  }

  pub fn lean_rpc_connect_request(&self, uri: &str) -> Json {
    let params = crate::messages::lean_rpc::connect_params(uri);

    self.request_with_id("$/lean/rpc/connect", &params).request
  }

  pub fn lean_rpc_get_plain_goals(&self, uri: &str, line: usize, character: usize) -> RequestWithId {
    let params = crate::messages::lean_rpc::get_plain_goals_params(uri, line, character);

    self.request_with_id("$/lean/plainGoal", &params)
  }
}
