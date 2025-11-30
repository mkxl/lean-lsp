use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use anyhow::Error as AnyhowError;
use mkutils::Utils;
use serde_json::Value as Json;

use crate::{session_runner::File, types::Utf16Position};

#[derive(Default)]
pub struct OpenFiles(HashMap<PathBuf, File>);

impl OpenFiles {
  pub fn contains(&self, filepath: &Path) -> bool {
    self.0.contains_key(filepath)
  }

  pub fn get_file(&self, filepath: &Path) -> Result<&File, AnyhowError> {
    self.0.get(filepath).context_path("file is not open", filepath)
  }

  pub fn get_file_mut(&mut self, filepath: &Path) -> Result<&mut File, AnyhowError> {
    self.0.get_mut(filepath).context_path("file is not open", filepath)
  }

  pub fn insert(&mut self, filepath: PathBuf, file: File) -> Option<File> {
    self.0.insert(filepath, file)
  }

  pub fn remove(&mut self, filepath: &Path) -> Option<File> {
    self.0.remove(filepath)
  }

  /// Mutates `message` by adding additional the following properties inside any
  /// (utf-16) [`Position`]:
  ///   - `character_bytes`: the character position in bytes.
  ///   - `character_utf8`: the character position in utf-8.
  ///   - `previous_line_length_bytes`: the length of the previous line in
  ///     bytes.
  ///   - `previous_line_length_utf8`: the length of the previous line in utf-8.
  ///
  /// This is done by looking at nearby `uri` or `textDocument.uri` properties,
  /// and using the open file as the source text to perform the utf-16 to utf-8
  /// conversions.
  ///
  /// For example,
  ///
  /// ```json
  /// {
  ///   "uri": "file:///some/file.txt",
  ///   "positions": [
  ///     {"line": 1, "character": 2}
  ///     // ...
  ///   ],
  ///   "object": {
  ///     "uri": "file:///some/other/file.txt",
  ///     "range": {
  ///       "start": {"line": 3, "character": 4},
  ///       "end": {"line": 5, "character": 6}
  ///     }
  ///   }
  /// }
  /// ```
  ///
  /// The `positions` array will have each of its utf-16 positions mutated to
  /// include the above additional properties, using `"file:///some/file.txt"` as
  /// the source.
  ///
  /// Similarly, the positions in `object.range` will be modified to include
  /// the above properties using `"file:///some/other/file.txt"` as the source.
  ///
  /// The above json would be transformed to
  ///
  /// ```json
  /// {
  ///   "uri": "file:///some/file.txt",
  ///   "positions": [
  ///     {
  ///       "line": 1,
  ///       "character": 2,
  ///       "character_bytes": ..,
  ///       "character_utf8": ..,
  ///       "previous_line_length_bytes": ..,
  ///       "previous_line_length_utf8": ..
  ///     }
  ///     // ...
  ///   ],
  ///   "object": {
  ///     "uri": "file:///some/other/file.txt",
  ///     "range": {
  ///       "start": {
  ///         "line": 3,
  ///         "character": 4,
  ///         "character_bytes": ..,
  ///         "character_utf8": ..,
  ///         "previous_line_length_bytes": ..,
  ///         "previous_line_length_utf8": ..
  ///       },
  ///       "end": {
  ///         "line": 5,
  ///         "character": 6,
  ///         "character_bytes": ..,
  ///         "character_utf8": ..,
  ///         "previous_line_length_bytes": ..,
  ///         "previous_line_length_utf8": ..
  ///       }
  ///     }
  ///   }
  /// }
  /// ```
  ///
  /// [`Position`]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position
  pub fn enrich_positions(&self, message: &mut Json) {
    self.enrich_positions_impl(message, None);
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
          .and_then(|uri| self.get_file(Path::new(&uri[7..])).ok())
          .map(|file| file.text().lines().collect());
        let lines = new_lines.as_deref().or(lines);

        for value in map.values_mut() {
          self.enrich_positions_impl(value, lines);
        }

        let line = map.get("line").and_then(Json::as_u64);
        let character = map.get("character").and_then(Json::as_u64);

        if let (Some(lines), Some(line), Some(character)) = (lines, line, character) {
          #[allow(clippy::cast_possible_truncation)]
          let utf16_position = Utf16Position::new(line as usize, character as usize);

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
}
