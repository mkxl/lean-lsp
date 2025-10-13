use std::path::PathBuf;

use anyhow::Error;
use mkutils::Utils;
use tokio::{sync::mpsc::UnboundedSender as MpscUnboundedSender, task::JoinHandle};
use ulid::Ulid;

use crate::{
  commands::{NewSessionCommand, SessionSetCommand},
  session::Session,
  session_set_runner::SessionSetRunner,
};

pub struct SessionSet {
  commands: MpscUnboundedSender<SessionSetCommand>,
  join_handle: JoinHandle<Result<(), Error>>,
}

impl SessionSet {
  pub fn new() -> Self {
    let (commands, runner_commands) = tokio::sync::mpsc::unbounded_channel();
    let join_handle = SessionSetRunner::new(runner_commands).run().spawn_task();

    Self { commands, join_handle }
  }

  #[tracing::instrument(skip_all)]
  pub async fn run_session(lean_path: PathBuf, lean_server_log_dirpath: Option<PathBuf>) -> Result<(), Error> {
    // NOTE: assign the result of [session_set.new_session()] to a variable to
    // prevent it from being immediately dropped and closing the associated
    // [SessionRunner] instances's commands receiver stream
    let session_set = Self::new();
    let _session = session_set.new_session(lean_path, lean_server_log_dirpath).await?;

    session_set.join_handle.await?
  }

  // TODO-8dffbb: extract out common functionality
  #[tracing::instrument(skip_all)]
  pub async fn new_session(
    &self,
    lean_path: PathBuf,
    lean_server_log_dirpath: Option<PathBuf>,
  ) -> Result<Session, Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let command = NewSessionCommand::new(lean_path, lean_server_log_dirpath);
    let new_session_command = SessionSetCommand::NewSession { sender, command };

    self.commands.send(new_session_command)?;

    receiver.await?
  }

  // TODO-8dffbb
  pub async fn get_sessions(&self) -> Result<Vec<Session>, Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_sessions = SessionSetCommand::GetSessions { sender };

    self.commands.send(get_sessions)?;

    receiver.await?.ok()
  }

  // TODO-8dffbb
  pub async fn get_session(&self, session_id: Option<Ulid>) -> Result<Session, Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_session = SessionSetCommand::GetSession { sender, session_id };

    self.commands.send(get_session)?;

    receiver.await?
  }
}

impl Default for SessionSet {
  fn default() -> Self {
    Self::new()
  }
}
