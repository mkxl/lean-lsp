use std::path::PathBuf;

use clap::Args;
use derive_more::{Constructor, From};
use mkutils::Utils;
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

#[derive(Constructor, Serialize)]
pub struct BytesPosition {
  pub line: usize,
  pub character: usize,
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

#[derive(Constructor, Serialize)]
pub struct Utf16Position {
  pub line: usize,
  pub character: usize,
}

impl Utf16Position {
  pub fn from_utf8(location: &Utf8Location, text: &str) -> Self {
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

  pub fn into_utf8_and_bytes(self, lines: &[&str]) -> Option<(Utf8Position, BytesPosition)> {
    let line_str = lines.get(self.line)?;
    let mut utf16_remaining = self.character;
    let mut utf8_offset = 0;
    let mut bytes_offset = 0;

    for c in line_str.chars() {
      let utf16_len = c.len_utf16();
      let bytes_len = c.len_utf8();

      if utf16_remaining < utf16_len {
        break;
      }

      utf16_remaining -= utf16_len;
      utf8_offset += 1;
      bytes_offset += bytes_len;
    }

    let utf8_position = Utf8Position::new(self.line, utf8_offset);
    let bytes_position = BytesPosition::new(self.line, bytes_offset);

    (utf8_position, bytes_position).some()
  }
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
