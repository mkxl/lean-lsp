use std::path::{Path, PathBuf};

use anyhow::Error as AnyhowError;
use mkutils::Utils;
use tokio::sync::mpsc::UnboundedSender as MpscUnboundedSender;
use ulid::Ulid;

use crate::{
  commands::SessionCommand,
  server::responses::GetNotificationsResponse,
  session_runner::SessionRunner,
  types::{GetPlainGoalsResult, Location, SessionStatus},
};

#[derive(Clone)]
pub struct Session {
  id: Ulid,
  commands: MpscUnboundedSender<SessionCommand>,
}

impl Session {
  pub fn new(
    lean_path: &Path,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<(Session, SessionRunner), AnyhowError> {
    let id = Ulid::new();
    let (commands, runner_commands) = tokio::sync::mpsc::unbounded_channel();
    let session_runner = SessionRunner::new(id, runner_commands, lean_path, lean_server_log_dirpath)?;
    let session = Session { id, commands };
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
  pub async fn get_plain_goals(&self, location: Location) -> Result<GetPlainGoalsResult, AnyhowError> {
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

  // TODO-8dffbb
  pub async fn notifications(&self) -> Result<GetNotificationsResponse, AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_notifications_command = SessionCommand::GetNotifications { sender };

    self.commands.send(get_notifications_command)?;

    receiver.await?.ok()
  }
}
