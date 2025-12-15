use enum_assoc::Assoc;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct FileProgress {}

#[derive(Clone, Deserialize, Serialize)]
pub struct PublishDiagnostics {}

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
