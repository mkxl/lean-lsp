use std::{
  fmt::{Debug, Display},
  marker::PhantomData,
};

use anyhow::{Context, Error as AnyhowError};
use camino::Utf8PathBuf;
use clap::Args;
use derive_more::{Constructor, Debug as DeriveMoreDebug};
use mkutils::Utils;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Deserialize, Serialize)]
pub struct PlainGoals {
  pub goals: Vec<String>,
  pub rendered: String,
}

#[derive(Clone, Copy)]
pub struct Utf8;

#[derive(Clone, Copy)]
pub struct Utf16;

#[derive(Clone, Copy)]
pub struct Bytes;

#[derive(Args, Clone, Copy, DeriveMoreDebug, Deserialize, Serialize)]
pub struct Position<T> {
  #[arg(long)]
  pub line: usize,

  #[arg(long)]
  pub character: usize,

  #[arg(skip)]
  #[debug(skip)]
  #[serde(skip)]
  phantom: PhantomData<T>,
}

impl<T> Position<T> {
  pub const fn new(line: usize, character: usize) -> Self {
    let phantom = PhantomData;

    Self {
      line,
      character,
      phantom,
    }
  }
}

impl Position<Utf16> {
  pub fn from_utf8(utf8_position: Position<Utf8>, text: &str) -> Result<Self, AnyhowError> {
    let line_str = text.lines().nth(utf8_position.line).context("no such line")?;
    let character = line_str
      .chars()
      .take(utf8_position.character)
      .map(char::len_utf16)
      .sum();
    let utf16_position = Self::new(utf8_position.line, character);

    utf16_position.ok()
  }

  pub fn into_utf8_and_bytes_positions(self, lines: &[&str]) -> Option<(Position<Utf8>, Position<Bytes>)> {
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

    let utf8_position = Position::<Utf8>::new(self.line, utf8_offset);
    let bytes_position = Position::<Bytes>::new(self.line, bytes_offset);

    (utf8_position, bytes_position).some()
  }
}

#[derive(Args, DeriveMoreDebug, Deserialize, Serialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct Location<T> {
  pub filepath: Utf8PathBuf,

  #[command(flatten)]
  #[serde(flatten)]
  pub position: Position<T>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct Range<T> {
  start: Position<T>,
  end: Position<T>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Processing {
  kind: u8,
  range: Range<Utf16>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct TextDocument {
  uri: String,
  version: usize,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Diagnostic {
  severity: u8,
  range: Range<Utf16>,
}

#[derive(Constructor, Deserialize, Serialize)]
pub struct SessionInfo {
  pub id: Ulid,
  pub project_dirpath: Utf8PathBuf,
}

#[derive(Constructor, Deserialize, Serialize)]
pub struct AppError {
  pub error: String,
  pub details: String,
}

impl<E: Debug + Display> From<E> for AppError {
  fn from(error: E) -> Self {
    Self::new(error.to_string(), error.debug().to_string())
  }
}

impl From<AppError> for AnyhowError {
  fn from(AppError { error, details }: AppError) -> Self {
    anyhow::anyhow!("{error} ({details})")
  }
}
