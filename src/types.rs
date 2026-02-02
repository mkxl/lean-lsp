use std::{
  fmt::{Debug, Display},
  marker::PhantomData,
};

use anyhow::{Context, Error as AnyhowError};
use camino::Utf8PathBuf;
use clap::Args;
use derive_more::{Constructor, Debug as DeriveMoreDebug};
use mkutils::{Default as MkutilsDefault, Utils};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use ulid::Ulid;

#[derive(Debug, Deserialize, Serialize)]
pub struct PlainGoals {
  pub goals: Vec<String>,
  pub rendered: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Utf8;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Utf16;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Bytes;

#[derive(Args, Clone, Copy, MkutilsDefault, DeriveMoreDebug, Deserialize, Serialize)]
pub struct Position<T> {
  #[arg(long)]
  pub line: usize,

  #[arg(long)]
  pub character: usize,

  #[arg(skip)]
  pub character_bytes: Option<usize>,

  #[arg(skip)]
  pub character_utf8: Option<usize>,

  #[arg(skip)]
  pub previous_line_length_bytes: Option<usize>,

  #[arg(skip)]
  pub previous_line_length_utf8: Option<usize>,

  #[arg(skip)]
  #[debug(skip)]
  #[serde(skip)]
  phantom: PhantomData<T>,
}

impl<T> Position<T> {
  pub fn new(line: usize, character: usize) -> Self {
    Self {
      line,
      character,
      ..Self::default()
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

#[derive(Args, Debug, Deserialize, Serialize)]
pub struct Location<T> {
  pub filepath: Utf8PathBuf,

  #[command(flatten)]
  #[serde(flatten)]
  pub position: Position<T>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Range<T> {
  start: Position<T>,
  end: Position<T>,
}

// NOTE: [https://leanprover-community.github.io/mathlib4_docs/Lean/Data/Lsp/Extra.html#Lean.Lsp.LeanFileProgressKind]
#[derive(Clone, Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum Kind {
  Processing = 1,
  FatalError = 2,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Processing {
  kind: Kind,
  range: Range<Utf16>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextDocument {
  uri: String,
  version: usize,
}

// NOTE: [https://leanprover-community.github.io/mathlib4_docs/Lean/Data/Lsp/Diagnostics.html#Lean.Lsp.DiagnosticSeverity]
#[derive(Clone, Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum Severity {
  Error = 1,
  Warning = 2,
  Information = 3,
  Hint = 4,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Diagnostic {
  pub severity: Severity,
  pub range: Range<Utf16>,
  pub message: String,
}

#[derive(Constructor, Debug, Deserialize, Serialize)]
pub struct SessionInfo {
  pub id: Ulid,
  pub project_dirpath: Utf8PathBuf,
}

// NOTE: [https://leanprover-community.github.io/mathlib4_docs/Lean/Data/Lsp/Extra.html#Lean.Lsp.RpcConnected]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcConnected {
  pub session_id: String,
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
