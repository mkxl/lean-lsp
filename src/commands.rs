use std::path::PathBuf;

use anyhow::Error as AnyhowError;
use clap::Args;
use derive_more::Constructor;
use mkutils::Utils;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::Sender as OneshotSender;
use ulid::Ulid;

use crate::{
  lean_server::LeanServer,
  server::responses::{GetPlainGoalsResponse, HoverFileResponse},
  session::Session,
  types::{SessionStatus, Utf8Location},
};

pub enum SessionCommand {
  Initialize {
    sender: OneshotSender<()>,
  },
  OpenFile {
    sender: OneshotSender<Result<(), AnyhowError>>,
    filepath: PathBuf,
  },
  ChangeFile {
    sender: OneshotSender<Result<(), AnyhowError>>,
    filepath: PathBuf,
    text: String,
  },
  CloseFile {
    sender: OneshotSender<Result<(), AnyhowError>>,
    filepath: PathBuf,
  },
  HoverFile {
    sender: OneshotSender<HoverFileResponse>,
    location: Utf8Location,
  },
  GetPlainGoals {
    sender: OneshotSender<GetPlainGoalsResponse>,
    location: Utf8Location,
  },
  GetStatus {
    sender: OneshotSender<SessionStatus>,
  },
  Kill {
    sender: OneshotSender<()>,
  },
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct NewSessionCommand {
  #[arg(default_value = Self::DEFAULT_LEAN_PATH_STR)]
  pub lean_path: PathBuf,

  #[arg(long)]
  pub lake_exe_path: Option<PathBuf>,

  #[arg(long = "log-dir", env = Self::LEAN_SERVER_LOG_DIRPATH_ENV_NAME)]
  pub lean_server_log_dirpath: Option<PathBuf>,

  #[arg(long)]
  pub enrich_utf16_positions: bool,
}

impl NewSessionCommand {
  const DEFAULT_LEAN_PATH_STR: &'static str = ".";
  const LEAN_SERVER_LOG_DIRPATH_ENV_NAME: &'static str = LeanServer::LOG_DIRPATH_ENV_NAME;

  pub fn absolute(&self) -> Result<Self, AnyhowError> {
    Self {
      lean_path: std::path::absolute(&self.lean_path)?,
      lake_exe_path: self.lake_exe_path.as_ref().map(std::path::absolute).transpose()?,
      lean_server_log_dirpath: self
        .lean_server_log_dirpath
        .as_ref()
        .map(std::path::absolute)
        .transpose()?,
      enrich_utf16_positions: self.enrich_utf16_positions,
    }
    .ok()
  }
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct OpenFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,
  pub lean_filepath: PathBuf,
}

#[derive(Args)]
pub struct ChangeFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,
  pub lean_filepath: PathBuf,
  #[arg(long)]
  pub input_filepath: Option<PathBuf>,
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct CloseFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,
  pub lean_filepath: PathBuf,
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct HoverFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,
  #[command(flatten)]
  pub location: Utf8Location,
}

pub enum SessionSetCommand {
  NewSession {
    sender: OneshotSender<Result<Session, AnyhowError>>,
    command: NewSessionCommand,
  },
  GetSessions {
    sender: OneshotSender<Vec<Session>>,
  },
  GetSession {
    sender: OneshotSender<Result<Session, AnyhowError>>,
    session_id: Option<Ulid>,
  },
  Kill {
    sender: OneshotSender<Result<(), AnyhowError>>,
  },
}
