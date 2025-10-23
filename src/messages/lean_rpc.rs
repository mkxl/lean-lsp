use serde_json::Value as Json;

pub fn connect_params(uri: &str) -> Json {
  serde_json::json!({"uri": uri})
}

pub fn get_plain_goals(uri: &str, line: usize, character: usize) -> Json {
  serde_json::json!({
    "textDocument": {
      "uri": uri,
    },
    "position": {
      "line": line,
      "character": character,
    },
  })
}
