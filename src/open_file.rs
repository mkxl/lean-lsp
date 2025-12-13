use std::collections::HashMap;

use anyhow::Error as AnyhowError;
use camino::{Utf8Path, Utf8PathBuf};
use mkutils::Utils;
use serde_json::Value as Json;

use crate::{
  message::text_document::INITIAL_VERSION,
  types::{Position, Utf16},
};

pub struct OpenFile {
  text: String,
  version: usize,
}

impl OpenFile {
  const INITIAL_VERSION: usize = INITIAL_VERSION;

  pub const fn new(text: String) -> Self {
    let version = Self::INITIAL_VERSION;

    Self { text, version }
  }

  pub fn text(&self) -> &str {
    &self.text
  }

  pub const fn increment_version(&mut self) -> usize {
    let version = self.version;

    self.version += 1;

    version
  }
}

#[derive(Default)]
pub struct OpenFileMap {
  open_files: HashMap<Utf8PathBuf, OpenFile>,
}

impl OpenFileMap {
  const FILE_NOT_OPEN_MESSAGE: &str = "file is not open";
  const FILE_ALREADY_OPEN_MESSAGE: &str = "file is already open";

  pub fn insert(&mut self, filepath: Utf8PathBuf, open_file: OpenFile) {
    self.open_files.insert(filepath, open_file);
  }

  pub fn get(&self, filepath: &Utf8Path) -> Result<&OpenFile, AnyhowError> {
    self
      .open_files
      .get(filepath)
      .context_path(Self::FILE_NOT_OPEN_MESSAGE, filepath)
  }

  pub fn get_mut(&mut self, filepath: &Utf8Path) -> Result<&mut OpenFile, AnyhowError> {
    self
      .open_files
      .get_mut(filepath)
      .context_path(Self::FILE_NOT_OPEN_MESSAGE, filepath)
  }

  pub fn check_contains(&self, filepath: &Utf8Path) -> Result<(), AnyhowError> {
    self.get(filepath)?.unit().ok()
  }

  pub fn check_doesnt_contain(&self, filepath: &Utf8Path) -> Result<(), AnyhowError> {
    if self.open_files.contains_key(filepath) {
      None.context_path(Self::FILE_ALREADY_OPEN_MESSAGE, filepath)
    } else {
      ().ok()
    }
  }

  pub fn remove(&mut self, filepath: &Utf8Path) {
    self.open_files.remove(filepath);
  }

  fn enrich_positions_impl(&self, message: &mut Json, lines: Option<&[&str]>) {
    match message {
      Json::Array(arr) => {
        for value in arr {
          self.enrich_positions_impl(value, lines);
        }
      }

      Json::Object(map) => {
        let uri = map.get("uri").and_then(Json::as_str);
        let text_document_uri = map
          .get("textDocument")
          .and_then(|doc| doc.as_object()?.get("uri")?.as_str());
        let new_lines: Option<Vec<&str>> = uri
          .or(text_document_uri)
          .and_then(|uri| self.get(Utf8Path::new(&uri[7..])).ok())
          .map(|open_file| open_file.text().lines().collect());
        let lines = new_lines.as_deref().or(lines);

        for value in map.values_mut() {
          self.enrich_positions_impl(value, lines);
        }

        let line = map.get("line").and_then(Json::as_u64);
        let character = map.get("character").and_then(Json::as_u64);

        if let (Some(lines), Some(line), Some(character)) = (lines, line, character) {
          #[allow(clippy::cast_possible_truncation)]
          let utf16_position = Position::<Utf16>::new(line as usize, character as usize);

          if let Some((utf8_position, bytes_position)) = utf16_position.into_utf8_and_bytes_positions(lines) {
            map.insert(
              "character_bytes".to_owned(),
              serde_json::json!(bytes_position.character),
            );
            map.insert("character_utf8".to_owned(), serde_json::json!(utf8_position.character));

            if utf16_position.character == 0
              && let Some(prev_line) = lines.get(utf16_position.line - 1)
            {
              let prev_line_len_bytes = prev_line.len();
              let prev_line_len_utf8 = prev_line.chars().count();

              map.insert(
                "previous_line_length_bytes".to_owned(),
                serde_json::json!(prev_line_len_bytes),
              );
              map.insert(
                "previous_line_length_utf8".to_owned(),
                serde_json::json!(prev_line_len_utf8),
              );
            }
          }
        }
      }

      _ => (),
    }
  }

  pub fn enrich_positions(&self, message: &mut Json) {
    self.enrich_positions_impl(message, None);
  }
}
