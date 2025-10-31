use std::path::PathBuf;

use anyhow::Error as AnyhowError;
use clap::Args;
use derive_more::Constructor;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;
use tokio::sync::oneshot::Sender as OneshotSender;
use ulid::Ulid;

use crate::{
  lean_server::LeanServer,
  session::Session,
  server::GetNotificationsResult,
  types::{GetPlainGoalsResult, Location, SessionStatus},
};

pub enum SessionCommand {
  Initialize {
    sender: OneshotSender<()>,
  },
  OpenFile {
    sender: OneshotSender<Result<(), AnyhowError>>,
    filepath: PathBuf,
  },
  GetPlainGoals {
    sender: OneshotSender<GetPlainGoalsResult>,
    location: Location,
  },
  GetStatus {
    sender: OneshotSender<SessionStatus>,
  }
  GetNotifications {
    sender: OneshotSender<GetNotificationsResult>,
  },
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct NewSessionCommand {
  #[arg(default_value = Self::DEFAULT_LEAN_PATH_STR)]
  pub lean_path: PathBuf,

  #[arg(long = "log-dir", env = Self::LEAN_SERVER_LOG_DIRPATH_ENV_NAME)]
  pub lean_server_log_dirpath: Option<PathBuf>,
}

impl NewSessionCommand {
  const DEFAULT_LEAN_PATH_STR: &'static str = ".";
  const LEAN_SERVER_LOG_DIRPATH_ENV_NAME: &'static str = LeanServer::LOG_DIRPATH_ENV_NAME;
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct OpenFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  pub lean_filepath: PathBuf,
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
}
