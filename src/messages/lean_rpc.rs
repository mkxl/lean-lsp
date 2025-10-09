use serde_json::Value as Json;

pub fn connect_params(uri: &str) -> Json {
  serde_json::json!({"uri": uri})
}
