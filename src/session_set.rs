use std::path::PathBuf;

use anyhow::Error as AnyhowError;
use mkutils::Utils;
use tokio::{sync::mpsc::UnboundedSender as MpscUnboundedSender, task::JoinHandle};
use ulid::Ulid;

use crate::{
  commands::{NewSessionCommand, SessionSetCommand},
  session::Session,
  session_set_runner::SessionSetRunner,
  types::SessionSetStatus,
};

pub struct SessionSet {
  commands: MpscUnboundedSender<SessionSetCommand>,
  join_handle: JoinHandle<Result<(), AnyhowError>>,
}

impl SessionSet {
  pub fn new() -> Self {
    let (commands, runner_commands) = tokio::sync::mpsc::unbounded_channel();
    let join_handle = SessionSetRunner::new(runner_commands).run().spawn_task();

    Self { commands, join_handle }
  }

  #[tracing::instrument(skip_all)]
  pub async fn new_session(
    &self,
    lean_path: PathBuf,
    lean_server_log_dirpath: Option<PathBuf>,
  ) -> Result<Session, AnyhowError> {
    let command = NewSessionCommand::new(lean_path, lean_server_log_dirpath);

    crate::macros::run_command!(self, SessionSetCommand::NewSession, command)
  }

  pub async fn get_sessions(&self) -> Result<Vec<Session>, AnyhowError> {
    crate::macros::run_command!(self, SessionSetCommand::GetSessions).ok()
  }

  pub async fn get_session(&self, session_id: Option<Ulid>) -> Result<Session, AnyhowError> {
    crate::macros::run_command!(self, SessionSetCommand::GetSession, session_id)
  }

  pub async fn status(&self) -> Result<SessionSetStatus, AnyhowError> {
    let session_set = self.join_handle.is_finished().into();
    let sessions = self
      .get_sessions()
      .await?
      .iter()
      .map(Session::status)
      .try_join_all()
      .await?;
    let session_set_status = SessionSetStatus::new(session_set, sessions);

    session_set_status.ok()
  }
}

impl Default for SessionSet {
  fn default() -> Self {
    Self::new()
  }
}
