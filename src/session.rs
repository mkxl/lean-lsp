use std::path::{Path, PathBuf};

use anyhow::Error as AnyhowError;
use mkutils::Utils;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender as MpscUnboundedSender;
use ulid::Ulid;

use crate::{
  commands::{GetPlainGoalsCommand, SessionCommand},
  lean_server::ProcessStatus,
  server::GetPlainGoalsResult,
  session_runner::SessionRunner,
};

#[derive(Deserialize, Object, Serialize)]
pub struct SessionStatus {
  id: Ulid,
  process: ProcessStatus,
}

#[derive(Clone)]
pub struct Session {
  id: Ulid,
  commands: MpscUnboundedSender<SessionCommand>,
}

impl Session {
  pub async fn new(
    lean_path: &Path,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<(Session, SessionRunner), AnyhowError> {
    let id = Ulid::new();
    let (commands, runner_commands) = tokio::sync::mpsc::unbounded_channel();
    let session_runner = SessionRunner::new(id, runner_commands, lean_path, lean_server_log_dirpath).await?;
    let session = Session { id, commands };
    let pair = session.pair(session_runner);

    pair.ok()
  }

  pub fn id(&self) -> Ulid {
    self.id
  }

  // TODO-8dffbb
  pub async fn open_file(&self, filepath: PathBuf) -> Result<(), AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let open_file_command = SessionCommand::OpenFile { sender, filepath };

    self.commands.send(open_file_command)?;

    receiver.await?
  }

  // TODO-8dffbb
  pub async fn get_plain_goals(
    &self,
    filepath: PathBuf,
    line: usize,
    character: usize,
  ) -> Result<GetPlainGoalsResult, AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let command = GetPlainGoalsCommand {
      filepath,
      line,
      character,
    };
    let get_pain_goals_command = SessionCommand::GetPlainGoals { sender, command };

    self.commands.send(get_pain_goals_command)?;

    receiver.await?.ok()
  }

  // TODO-8dffbb
  pub async fn status(&self) -> Result<SessionStatus, AnyhowError> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_process_status = SessionCommand::GetProcessStatus { sender };

    self.commands.send(get_process_status)?;

    let id = self.id;
    let process = receiver.await?;
    let session_status = SessionStatus { id, process };

    session_status.ok()
  }
}
