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

impl Utf8Location {
  pub fn into_utf16(self, text: &str) -> Utf16Location {
    let line_str = text.lines().nth(self.line).unwrap_or("");
    let utf16_offset = line_str
      .chars()
      .take(self.character)
      .map(char::len_utf16)
      .sum();

    Utf16Location {
      filepath: self.filepath,
      line: self.line,
      character: utf16_offset,
    }
  }
}

#[derive(Deserialize, Serialize)]
pub struct Utf16Location {
  pub filepath: PathBuf,
  pub line: usize,
  pub character: usize,
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
