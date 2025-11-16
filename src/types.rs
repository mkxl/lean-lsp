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
pub struct Utf8Location {
  pub filepath: PathBuf,

  #[arg(long)]
  pub line: usize,

  #[arg(long)]
  pub character: usize,
}

// No Constructor derive. UTF-16 locations can only be created from a UTF-8
// location, or deserialized from a server message.
#[derive(Deserialize, Serialize)]
pub struct Utf16Location {
  pub filepath: PathBuf,
  pub line: usize,
  pub character: usize,
}

impl Utf16Location {
  pub fn new(location: Utf8Location, text: &str) -> Self {
    let line_str = text.lines().nth(location.line).unwrap_or_default();
    let utf16_offset = line_str.chars().take(location.character).map(char::len_utf16).sum();

    Self {
      filepath: location.filepath,
      line: location.line,
      character: utf16_offset,
    }
  }
}

#[derive(Deserialize, Object, Serialize)]
pub struct PlainGoals {
  pub goals: Vec<String>,
  pub rendered: String,
}

#[derive(Deserialize, Object, Serialize)]
pub struct SessionStatus {
  pub id: Ulid,
  pub process: TaskStatus,
  pub project_dirpath: PathBuf,
}

#[derive(Constructor, Deserialize, Object, Serialize)]
pub struct SessionSetStatus {
  session_set: TaskStatus,
  sessions: Vec<SessionStatus>,
}
