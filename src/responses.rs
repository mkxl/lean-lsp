use serde::{Deserialize, Serialize};

use crate::types::{HoverFileResult, PlainGoals};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HoverFileResponse {
  pub result: Option<HoverFileResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GetPlainGoalsResponse {
  pub result: Option<PlainGoals>,
}
