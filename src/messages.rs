pub mod initialize;
pub mod lean_rpc;
pub mod text_document;

use std::{path::Path, sync::atomic::AtomicUsize};

use mkutils::Utils;
use serde_json::Value as Json;

#[derive(Default)]
pub struct Messages {
  id: AtomicUsize,
}

impl Messages {
  fn request(&self, method: &str, params: &Json) -> Json {
    serde_json::json!({
      "jsonrpc": "2.0",
      "id": self.id.inc(),
      "method": method,
      "params": params,
    })
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

    self.request("initialize", &params)
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

    self.request("textDocument/documentSymbol", &params)
  }

  pub fn text_document_document_code_action_request(&self, uri: &str) -> Json {
    let params = crate::messages::text_document::document_code_action_params(uri);

    self.request("textDocument/codeAction", &params)
  }

  pub fn text_document_folding_range_request(&self, uri: &str) -> Json {
    let params = crate::messages::text_document::folding_range_params(uri);

    self.request("textDocument/foldingRange", &params)
  }

  pub fn lean_rpc_connect_request(&self, uri: &str) -> Json {
    let params = crate::messages::lean_rpc::connect_params(uri);

    self.request("$/lean/rpc/connect", &params)
  }
}
