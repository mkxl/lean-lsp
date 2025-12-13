use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::types::PlainGoals;

#[derive(Deserialize, Serialize)]
pub struct HoverFileResponse {
  pub result: Json,
}

#[derive(Deserialize, Serialize)]
pub struct GetPlainGoalsResponse {
  pub result: Option<PlainGoals>,
}
