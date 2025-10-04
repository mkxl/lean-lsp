use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use anyhow::Error;
use clap::Args;
use derive_more::Constructor;
use mkutils::{IntoStream, Utils};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use tokio::{
  sync::{mpsc::UnboundedSender as UnboundedMpscSender, oneshot::Sender as OneshotSender},
  task::JoinSet,
};
use tokio_stream::wrappers::UnboundedReceiverStream as UnboundedMpscReceiverStream;
use ulid::Ulid;

use crate::{
  lean_server::LeanServer,
  session::{Session, SessionClient},
};

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

  pub async fn run(self) -> Result<(), Error> {
    SessionSet::run_session(self.lean_path, self.lean_server_log_dirpath).await
  }
}

#[derive(Args, Constructor, Deserialize, Object, Serialize)]
pub struct OpenFileCommand {
  #[arg(long)]
  pub session_id: Option<Ulid>,

  pub lean_filepath: PathBuf,
}

pub enum SessionSetCommand {
  NewSession {
    sender: OneshotSender<Result<SessionClient, Error>>,
    command: NewSessionCommand,
  },
  GetSessionClients {
    sender: OneshotSender<Vec<SessionClient>>,
  },
  GetSessionClient {
    sender: OneshotSender<Result<SessionClient, Error>>,
    session_id: Option<Ulid>,
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
  pub async fn get_session_clients(&self) -> Result<Vec<SessionClient>, Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_session_clients = SessionSetCommand::GetSessionClients { sender };

    self.sender.send(get_session_clients)?;

    receiver.await?.ok()
  }

  // TODO-8dffbb
  pub async fn get_session_client(&self, session_id: Option<Ulid>) -> Result<SessionClient, Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let get_session_client = SessionSetCommand::GetSessionClient { sender, session_id };

    self.sender.send(get_session_client)?;

    receiver.await?
  }
}

pub struct SessionSet {
  commands: UnboundedMpscReceiverStream<SessionSetCommand>,
  session_clients: HashMap<Ulid, SessionClient>,
  session_run_join_set: JoinSet<(Ulid, Result<(), Error>)>,
}

impl SessionSet {
  pub fn new() -> (Self, SessionSetClient) {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let commands = receiver.into_stream();
    let session_clients = HashMap::new();
    let session_run_join_set = JoinSet::new();
    let session_set = Self {
      commands,
      session_clients,
      session_run_join_set,
    };
    let session_set_client = SessionSetClient::new(sender);

    (session_set, session_set_client)
  }

  async fn new_session(
    &mut self,
    lean_path: &Path,
    lean_server_log_dirpath: Option<&Path>,
  ) -> Result<SessionClient, Error> {
    let (session, session_client) = Session::new(lean_path, lean_server_log_dirpath).await?;
    let session_id = session.id();
    let session_run_future = async move { (session.id(), session.run().await) };

    self.session_clients.insert(session_id, session_client.clone());
    self.session_run_join_set.spawn(session_run_future);

    session_client.ok()
  }

  fn get_session_clients(&self) -> Vec<SessionClient> {
    self.session_clients.values().cloned().collect()
  }

  fn get_session_client(&self, session_id: Option<Ulid>) -> Result<SessionClient, Error> {
    if let Some(session_id) = session_id {
      self.session_clients.try_get(&session_id)?.clone().ok()
    } else if self.session_clients.len() == 1 {
      self.session_clients.values().next_item()?.clone().ok()
    } else {
      anyhow::bail!("unspecified session id is ambiguous")
    }
  }

  #[tracing::instrument(skip_all)]
  async fn process_command(&mut self, command: SessionSetCommand) -> Result<(), Error> {
    match command {
      SessionSetCommand::NewSession { sender, command } => self
        .new_session(command.lean_path.as_ref(), command.lean_server_log_dirpath.map_as_ref())
        .await
        .send_to_oneshot(sender)?,
      SessionSetCommand::GetSessionClients { sender } => self.get_session_clients().send_to_oneshot(sender)?,
      SessionSetCommand::GetSessionClient { sender, session_id } => {
        self.get_session_client(session_id).send_to_oneshot(sender)?;
      }
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
        session_set_command_res = self.commands.next_item_async() => self.process_command(session_set_command_res?).await.log_error("error processing command").unit(),
        session_id_and_run_result = self.session_run_join_set.join_next() => match session_id_and_run_result {
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
    let session_set_handle = session_set.run().spawn_task();

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

    session_set_handle.await?
  }
}
