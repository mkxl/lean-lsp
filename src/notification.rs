use enum_assoc::Assoc;
use serde::{Deserialize, Serialize};

use crate::types::{Diagnostic, Processing, TextDocument};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileProgress {
  pub processing: Vec<Processing>,
  pub text_document: TextDocument,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PublishDiagnostics {
  pub diagnostics: Vec<Diagnostic>,

  #[serde(flatten)]
  pub text_document: TextDocument,
}

#[derive(Assoc, Clone, Deserialize, Serialize)]
#[serde(tag = "method", content = "params")]
#[func(pub fn method(&self) -> &'static str)]
pub enum Notification {
  #[serde(rename = "$/lean/fileProgress")]
  #[assoc(method = "$/lean/fileProgress")]
  FileProgress(FileProgress),

  #[serde(rename = "textDocument/publishDiagnostics")]
  #[assoc(method = "textDocument/publishDiagnostics")]
  PublishDiagnostics(PublishDiagnostics),

  #[serde(other)]
  #[assoc(method = "other")]
  Other,
}
