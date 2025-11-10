use derive_more::From;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use ulid::Ulid;

use crate::types::{PlainGoals, SessionStatus};

#[derive(From, Deserialize, Object, Serialize)]
pub struct NewSessionResponse {
  pub session_id: Ulid,
}

#[derive(Deserialize, From, Object)]
pub struct GetSessionsResponse {
  pub sessions: Vec<SessionStatus>,
}

#[derive(Deserialize, Object, Serialize)]
pub struct GetPlainGoalsResponse {
  pub result: Option<PlainGoals>,
}

#[derive(Deserialize, Object, Serialize)]
pub struct HoverFileResponse {
  pub result: Json,
}
