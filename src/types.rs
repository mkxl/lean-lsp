use std::path::PathBuf;

use clap::Args;
use derive_more::{Constructor, From};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Deserialize, Object, Serialize)]
pub struct PlainGoals {
  pub goals: Vec<String>,
  pub rendered: String,
}

#[derive(Deserialize, From, Object, Serialize)]
pub struct TaskStatus {
  pub is_finished: bool,
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct Utf8Position {
  #[arg(long)]
  pub line: usize,
  #[arg(long)]
  pub character: usize,
}

#[derive(Args, Object, Deserialize, Serialize)]
pub struct Utf8Location {
  pub filepath: PathBuf,

  #[command(flatten)]
  #[serde(flatten)]
  pub position: Utf8Position,
}

impl Utf8Location {
  pub fn new(filepath: PathBuf, line: usize, character: usize) -> Self {
    let position = Utf8Position::new(line, character);

    Self { filepath, position }
  }
}

#[derive(Deserialize, Serialize)]
pub struct Utf16Position {
  pub line: usize,
  pub character: usize,
}

impl Utf16Position {
  pub fn new(location: &Utf8Location, text: &str) -> Self {
    let line_str = text.lines().nth(location.position.line).unwrap_or_default();
    let utf16_offset = line_str
      .chars()
      .take(location.position.character)
      .map(char::len_utf16)
      .sum();

    Self {
      line: location.position.line,
      character: utf16_offset,
    }
  }
}

#[derive(Deserialize, Serialize)]
pub struct Utf16Range {
  pub start: Utf16Position,
  pub end: Utf16Position,
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
