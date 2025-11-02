use std::path::{Path, PathBuf};

use anyhow::Error as AnyhowError;
use mkutils::{IntoStream, Utils};
use serde_json::Value as Json;
use tokio::sync::{broadcast::Sender as BroadcastSender, mpsc::UnboundedSender as MpscUnboundedSender};
use tokio_stream::wrappers::BroadcastStream as BroadcastReceiverStream;
use ulid::Ulid;

use crate::{
  commands::SessionCommand,
  server::responses::GetPlainGoalsResponse,
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

  // TODO-8dffbb
  pub async fn initialize(&self) -> Result<(), AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let initialize_command = SessionCommand::Initialize { sender };

    self.commands.send(initialize_command)?;

    receiver.await?.ok()
  }

  // TODO-8dffbb
  pub async fn open_file(&self, filepath: PathBuf) -> Result<(), AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let open_file_command = SessionCommand::OpenFile { sender, filepath };

    self.commands.send(open_file_command)?;

    receiver.await?
  }

  // TODO-8dffbb
  pub async fn change_file(&self, filepath: PathBuf, text: String) -> Result<(), AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let change_file_command = SessionCommand::ChangeFile { sender, filepath, text };

    self.commands.send(change_file_command)?;

    receiver.await?
  }

  // TODO-8dffbb
  pub async fn close_file(&self, filepath: PathBuf) -> Result<(), AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let close_file_command = SessionCommand::CloseFile { sender, filepath };

    self.commands.send(close_file_command)?;

    receiver.await?
  }

  // TODO-8dffbb
  pub async fn get_plain_goals(&self, location: Location) -> Result<GetPlainGoalsResponse, AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_plain_goals_command = SessionCommand::GetPlainGoals { sender, location };

    self.commands.send(get_plain_goals_command)?;

    receiver.await?.ok()
  }

  // TODO-8dffbb
  pub async fn status(&self) -> Result<SessionStatus, AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_process_status = SessionCommand::GetStatus { sender };

    self.commands.send(get_process_status)?;

    receiver.await?.ok()
  }

  pub fn notifications(&self) -> BroadcastReceiverStream<Json> {
    self.notifications.subscribe().into_stream()
  }
}
