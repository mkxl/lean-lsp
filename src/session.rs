use std::path::{Path, PathBuf};

use anyhow::Error as AnyhowError;
use mkutils::{IntoStream, Utils};
use serde_json::Value as Json;
use tokio::sync::{broadcast::Sender as BroadcastSender, mpsc::UnboundedSender as MpscUnboundedSender};
use tokio_stream::wrappers::BroadcastStream as BroadcastReceiverStream;
use ulid::Ulid;

use crate::{
  commands::SessionCommand,
  server::responses::{GetPlainGoalsResponse, HoverFileResponse},
  session_runner::SessionRunner,
  types::{Location, SessionStatus},
};

#[derive(Clone)]
pub struct Session {
  id: Ulid,
  commands: MpscUnboundedSender<SessionCommand>,
  notifications: BroadcastSender<Json>,
}

impl Session {
  const NOTIFICATIONS_CAPACITY: usize = 32;

  pub fn new(
    lean_path: &Path,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<(Session, SessionRunner), AnyhowError> {
    let id = Ulid::new();
    let (commands, runner_commands) = tokio::sync::mpsc::unbounded_channel();
    let (notifications, _notifications_receiver) = tokio::sync::broadcast::channel(Self::NOTIFICATIONS_CAPACITY);
    let session_runner = SessionRunner::new(
      id,
      runner_commands,
      notifications.clone(),
      lean_path,
      lean_server_log_dirpath,
    )?;
    let session = Session {
      id,
      commands,
      notifications,
    };
    let pair = session.pair(session_runner);

    pair.ok()
  }

  pub fn id(&self) -> Ulid {
    self.id
  }

  pub async fn initialize(&self) -> Result<(), AnyhowError> {
    crate::macros::run_command!(self, SessionCommand::Initialize).ok()
  }

  pub async fn open_file(&self, filepath: PathBuf) -> Result<(), AnyhowError> {
    crate::macros::run_command!(self, SessionCommand::OpenFile, filepath)
  }

  pub async fn change_file(&self, filepath: PathBuf, text: String) -> Result<(), AnyhowError> {
    crate::macros::run_command!(self, SessionCommand::ChangeFile, filepath, text)
  }

  pub async fn close_file(&self, filepath: PathBuf) -> Result<(), AnyhowError> {
    crate::macros::run_command!(self, SessionCommand::CloseFile, filepath)
  }

  pub async fn hover_file(&self, location: Location) -> Result<HoverFileResponse, AnyhowError> {
    crate::macros::run_command!(self, SessionCommand::HoverFile, location).ok()
  }

  pub async fn get_plain_goals(&self, location: Location) -> Result<GetPlainGoalsResponse, AnyhowError> {
    crate::macros::run_command!(self, SessionCommand::GetPlainGoals, location).ok()
  }

  pub async fn status(&self) -> Result<SessionStatus, AnyhowError> {
    crate::macros::run_command!(self, SessionCommand::GetStatus).ok()
  }

  pub fn notifications(&self) -> BroadcastReceiverStream<Json> {
    self.notifications.subscribe().into_stream()
  }

  pub async fn kill(&self) -> Result<(), AnyhowError> {
    crate::macros::run_command!(self, SessionCommand::Kill)
  }
}
