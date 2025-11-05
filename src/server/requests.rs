use std::path::PathBuf;

use anyhow::Error as AnyhowError;
use mkutils::Utils;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::commands::ChangeFileCommand;

#[derive(Deserialize, Object, Serialize)]
pub struct ChangeFileRequest {
  pub session_id: Option<Ulid>,
  pub lean_filepath: PathBuf,
  pub text: String,
}

impl ChangeFileRequest {
  pub async fn new(command: ChangeFileCommand) -> Result<Self, AnyhowError> {
    let session_id = command.session_id;
    let lean_filepath = command.lean_filepath;
    let text = match command.input_filepath {
      Some(input_filepath) => {
        input_filepath
          .open_async()
          .await?
          .buf_reader_async()
          .read_string_async()
          .await?
      }
      None => tokio::io::stdin().buf_reader_async().read_string_async().await?,
    };
    let change_file_request = Self {
      session_id,
      lean_filepath,
      text,
    };

    change_file_request.ok()
  }
}
