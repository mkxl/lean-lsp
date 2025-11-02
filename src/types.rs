use std::path::PathBuf;

use clap::Args;
use derive_more::{Constructor, From};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Deserialize, From, Object, Serialize)]
pub struct TaskStatus {
  pub is_finished: bool,
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct Location {
  pub filepath: PathBuf,

  #[arg(long)]
  pub line: usize,

  #[arg(long)]
  pub character: usize,
}

#[derive(Deserialize, Object, Serialize)]
pub struct PlainGoals {
  pub goals: Vec<String>,
  pub rendered: String,
}

#[derive(Deserialize, Object, Serialize)]
pub struct GetPlainGoalsResult {
  pub result: Option<PlainGoals>,
}

#[derive(Deserialize, Object, Serialize)]
pub struct SessionStatus {
  pub id: Ulid,
  pub process: TaskStatus,
}

#[derive(Constructor, Deserialize, Object, Serialize)]
pub struct SessionSetStatus {
  session_set: TaskStatus,
  sessions: Vec<SessionStatus>,
}
