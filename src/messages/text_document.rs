use serde_json::Value as Json;

const LEAN_LANGUAGE_ID: &str = "lean4";
const INITIAL_TEXT_DOCUMENT_VERSION: usize = 0;

pub fn did_open_notification_params(text: &str, uri: &str) -> Json {
  serde_json::json!({
      "dependencyBuildMode": "never",
      "textDocument": {
          "languageId": LEAN_LANGUAGE_ID,
          "text": text,
          "uri": uri,
          "version": INITIAL_TEXT_DOCUMENT_VERSION,
      },
  })
}

pub fn document_symbol_params(uri: &str) -> Json {
  serde_json::json!({"textDocument": {"uri": uri}})
}

pub fn document_code_action_params(uri: &str) -> Json {
  serde_json::json!({
      "context": {"diagnostics": [], "triggerKind": 2},
      "range": {
          "end": {"character": 0, "line": 0},
          "start": {"character": 0, "line": 0},
      },
      "textDocument": {"uri": uri},
  })
}

pub fn folding_range_params(uri: &str) -> Json {
  serde_json::json!({"textDocument": {"uri": uri}})
}
