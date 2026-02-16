use serde::{Deserialize, Serialize};

use crate::types::{HoverFileResult, PlainGoals};

#[derive(Debug, Deserialize, Serialize)]
pub struct HoverFileResponse {
  pub result: HoverFileResult,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetPlainGoalsResponse {
  pub result: Option<PlainGoals>,
}
