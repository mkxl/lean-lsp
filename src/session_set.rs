use std::path::PathBuf;

use anyhow::Error as AnyhowError;
use derive_more::From;
use mkutils::Utils;
use tokio::{sync::mpsc::UnboundedSender as MpscUnboundedSender, task::JoinHandle};
use ulid::Ulid;

use crate::{
  commands::{NewSessionCommand, SessionSetCommand},
  session::Session,
  session_set_runner::SessionSetRunner,
};

#[derive(Clone, From)]
pub struct SessionSet {
  commands: MpscUnboundedSender<SessionSetCommand>,
}

impl SessionSet {
  pub fn new() -> (Self, JoinHandle<Result<(), AnyhowError>>) {
    let (commands, runner_commands) = tokio::sync::mpsc::unbounded_channel();
    let join_handle = SessionSetRunner::new(runner_commands).run().spawn_task();

    commands.convert::<Self>().pair(join_handle)
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
}
