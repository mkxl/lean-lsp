use std::{collections::HashMap, path::Path};

use anyhow::Error as AnyhowError;
use mkutils::{IntoStream, Utils};
use tokio::{sync::mpsc::UnboundedReceiver as MpscUnboundedReceiver, task::JoinSet};
use tokio_stream::wrappers::UnboundedReceiverStream as MpscUnboundedReceiverStream;
use ulid::Ulid;

use crate::{commands::SessionSetCommand, session::Session, session_runner::SessionResult};

pub struct SessionSetRunner {
  commands: MpscUnboundedReceiverStream<SessionSetCommand>,
  sessions: HashMap<Ulid, Session>,
  session_results: JoinSet<SessionResult>,
}

impl SessionSetRunner {
  pub fn new(commands: MpscUnboundedReceiver<SessionSetCommand>) -> Self {
    let commands = commands.into_stream();
    let sessions = HashMap::new();
    let session_results = JoinSet::new();

    Self {
      commands,
      sessions,
      session_results,
    }
  }

  async fn new_session(
    &mut self,
    lean_path: &Path,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<Session, AnyhowError> {
    let (session, session_runner) = Session::new(lean_path, lean_server_log_dirpath).await?;

    self.sessions.insert(session.id(), session.clone());
    self.session_results.spawn(session_runner.run());

    session.ok()
  }

  fn get_sessions(&self) -> Vec<Session> {
    self.sessions.values().cloned().collect()
  }

  fn get_session(&self, session_id: Option<Ulid>) -> Result<Session, AnyhowError> {
    if let Some(session_id) = session_id {
      self.sessions.try_get(&session_id)?.clone().ok()
    } else if self.sessions.len() == 1 {
      self.sessions.values().next_item()?.clone().ok()
    } else {
      anyhow::bail!("unspecified session id is ambiguous")
    }
  }

  #[tracing::instrument(skip_all)]
  async fn process_command(&mut self, command: SessionSetCommand) -> Result<(), AnyhowError> {
    match command {
      SessionSetCommand::NewSession { sender, command } => self
        .new_session(command.lean_path.as_ref(), command.lean_server_log_dirpath.map_as_ref())
        .await
        .send_to_oneshot(sender)?,
      SessionSetCommand::GetSessions { sender } => self.get_sessions().send_to_oneshot(sender)?,
      SessionSetCommand::GetSession { sender, session_id } => {
        self.get_session(session_id).send_to_oneshot(sender)?;
      }
    }

    ().ok()
  }

  #[tracing::instrument(skip_all)]
  fn cleanup_session(&mut self, session_result: SessionResult) {
    self.sessions.remove(&session_result.id);

    if let Err(error) = session_result.result {
      tracing::warn!(%error, "error running session");
    }

    tracing::info!(session_id = %session_result.id, "cleaned up session");
  }

  // TODO-8dffbb
  #[tracing::instrument(skip_all)]
  pub async fn run(mut self) -> Result<(), AnyhowError> {
    loop {
      tokio::select! {
        session_set_command_res = self.commands.next_item_async() => self.process_command(session_set_command_res?).await.log_error("error processing command").unit(),
        session_result_res = self.session_results.join_next() => match session_result_res {
          Some(Ok(session_result)) => self.cleanup_session(session_result),
          Some(Err(join_error)) => tracing::warn!(%join_error, "session run task failed to execute to completion"),
          None => tokio::task::yield_now().await,
        }
      }
    }
  }
}
