use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use anyhow::Error;
use derive_more::Constructor;
use mkutils::{IntoStream, Utils};
use poem_openapi::Object;
use tokio::{
  sync::{mpsc::UnboundedSender as UnboundedMpscSender, oneshot::Sender as OneshotSender},
  task::JoinSet,
};
use tokio_stream::wrappers::UnboundedReceiverStream as UnboundedMpscReceiverStream;
use ulid::Ulid;

use crate::session::{Session, SessionClient};

#[derive(Constructor, Object)]
pub struct NewSessionCommand {
  pub lean_path: PathBuf,
  pub lean_server_log_dirpath: Option<PathBuf>,
}

pub enum SessionSetCommand {
  NewSession {
    sender: OneshotSender<Result<SessionClient, Error>>,
    command: NewSessionCommand,
  },
  GetSessions {
    sender: OneshotSender<Vec<SessionClient>>,
  },
}

#[derive(Constructor)]
pub struct SessionSetClient {
  sender: UnboundedMpscSender<SessionSetCommand>,
}

impl SessionSetClient {
  // TODO-8dffbb: extract out common functionality
  #[tracing::instrument(skip_all)]
  pub async fn new_session(
    &self,
    lean_path: PathBuf,
    lean_server_log_dirpath: Option<PathBuf>,
  ) -> Result<SessionClient, Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let command = NewSessionCommand::new(lean_path, lean_server_log_dirpath);
    let new_session_command = SessionSetCommand::NewSession { sender, command };

    self.sender.send(new_session_command)?;

    receiver.await?
  }

  // TODO-8dffbb
  pub async fn get_sessions(&self) -> Result<Vec<SessionClient>, Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_sessions = SessionSetCommand::GetSessions { sender };

    self.sender.send(get_sessions)?;

    receiver.await?.ok()
  }
}

pub struct SessionSet {
  commands: UnboundedMpscReceiverStream<SessionSetCommand>,
  session_clients: HashMap<Ulid, SessionClient>,
  session_run_task_join_set: JoinSet<(Ulid, Result<(), Error>)>,
}

impl SessionSet {
  pub fn new() -> (Self, SessionSetClient) {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let commands = receiver.into_stream();
    let session_clients = HashMap::new();
    let session_run_task_join_set = JoinSet::new();
    let session_set = Self {
      commands,
      session_clients,
      session_run_task_join_set,
    };
    let session_set_client = SessionSetClient::new(sender);

    (session_set, session_set_client)
  }

  fn new_session(&mut self, lean_path: &Path, lean_server_log_dirpath: Option<&Path>) -> Result<SessionClient, Error> {
    let (session, session_client) = Session::new(lean_path, lean_server_log_dirpath)?;
    let session_id = session.id();
    let session_run_future = async move { (session.id(), session.run().await) };

    self.session_clients.insert(session_id, session_client.clone());
    self.session_run_task_join_set.spawn(session_run_future);

    session_client.ok()
  }

  fn get_sessions(&self) -> Vec<SessionClient> {
    self.session_clients.values().cloned().collect()
  }

  #[allow(clippy::unused_async)]
  #[tracing::instrument(skip_all)]
  async fn process_command(&mut self, command: SessionSetCommand) -> Result<(), Error> {
    match command {
      SessionSetCommand::NewSession { sender, command } => self
        .new_session(command.lean_path.as_ref(), command.lean_server_log_dirpath.map_as_ref())
        .send_to_oneshot(sender)?,
      SessionSetCommand::GetSessions { sender } => self.get_sessions().send_to_oneshot(sender)?,
    }

    ().ok()
  }

  fn cleanup_session(&mut self, session_id: Ulid, result: Result<(), Error>) {
    self.session_clients.remove(&session_id);

    if let Err(session_error) = result {
      tracing::warn!(%session_error, "error running session");
    }

    tracing::info!(%session_id, "cleaned up session");
  }

  // TODO-8dffbb
  #[tracing::instrument(skip_all)]
  pub async fn run(mut self) -> Result<(), Error> {
    loop {
      tokio::select! {
        session_set_command_res = self.commands.next_item() => self.process_command(session_set_command_res?).await.log_error("error processing command").unit(),
        session_id_and_run_result = self.session_run_task_join_set.join_next() => match session_id_and_run_result {
          Some(Ok((session_id, session_run_result))) => self.cleanup_session(session_id, session_run_result),
          Some(Err(join_error)) => tracing::warn!(%join_error, "session run task failed to execute to completion"),
          None => tokio::task::yield_now().await,
        }
      }
    }
  }

  #[tracing::instrument(skip_all)]
  pub async fn run_session(lean_path: PathBuf, lean_server_log_dirpath: Option<PathBuf>) -> Result<(), Error> {
    let (session_set, session_set_client) = Self::new();
    let session_set_run_task = session_set.run().spawn_task();

    // NOTE-97a211:
    // - if [session_set.run()] is not already running, then
    //   [session_set_client.new_session()] will hang, as it is the
    //   [session_set.run()] that processes the issued [NewSession] command
    // - assign the result of [session_set_client.new_session()] to a variable to
    //   prevent it from being immediately dropped and closing the associated
    //   [Session] object's receiver
    let _session_client = session_set_client
      .new_session(lean_path, lean_server_log_dirpath)
      .await?;

    session_set_run_task.await?
  }
}
