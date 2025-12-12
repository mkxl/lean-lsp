use serde_json::Value as Json;

use crate::types::{Position, Utf16};

pub fn connect_params(uri: &str) -> Json {
  serde_json::json!({"uri": uri})
}

pub fn get_plain_goals_params(uri: &str, position: Position<Utf16>) -> Json {
  serde_json::json!({
    "textDocument": {
      "uri": uri,
    },
    "position": position,
  })
}
