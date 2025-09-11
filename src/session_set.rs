use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use anyhow::Error;
use derive_more::Constructor;
use tokio::{
  sync::{
    mpsc::{Receiver as MpscReceiver, Sender as MpscSender},
    oneshot::Sender as OneshotSender,
  },
  task::JoinSet,
};
use ulid::Ulid;

use crate::{
  session::{Session, SessionClient},
  utils::Utils,
};

pub enum SessionSetCommand {
  NewSession {
    sender: OneshotSender<SessionClient>,
    lean_path: PathBuf,
    lean_server_log_dirpath: Option<PathBuf>,
  },
}

#[derive(Constructor)]
pub struct SessionSetClient {
  sender: MpscSender<SessionSetCommand>,
}

impl SessionSetClient {
  // TODO-8dffbb: extract out common functionality
  pub async fn new_session(
    &self,
    lean_path: PathBuf,
    lean_server_log_dirpath: Option<PathBuf>,
  ) -> Result<SessionClient, Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let new_session_command = SessionSetCommand::NewSession {
      sender,
      lean_path,
      lean_server_log_dirpath,
    };

    self.sender.send(new_session_command).await?;

    receiver.await?.ok()
  }
}

pub struct SessionSet {
  receiver: MpscReceiver<SessionSetCommand>,
  session_clients: HashMap<Ulid, SessionClient>,
  join_set: JoinSet<(Ulid, Result<(), Error>)>,
}

impl SessionSet {
  const COMMAND_CHANNEL_BUFFER_SIZE: usize = 64;

  pub fn new() -> (Self, SessionSetClient) {
    let (sender, receiver) = tokio::sync::mpsc::channel(Self::COMMAND_CHANNEL_BUFFER_SIZE);
    let session_clients = HashMap::new();
    let join_set = JoinSet::new();
    let session_set = Self {
      receiver,
      session_clients,
      join_set,
    };
    let session_set_client = SessionSetClient::new(sender);

    (session_set, session_set_client)
  }

  fn new_session(&mut self, lean_path: &Path, lean_server_log_dirpath: Option<&Path>) -> Result<SessionClient, Error> {
    let (session, session_client) = Session::new(lean_path, lean_server_log_dirpath)?;
    let session_id = session.id();
    let run_future = async move { (session.id(), session.run().await) };

    self.session_clients.insert(session_id, session_client.clone());
    self.join_set.spawn(run_future);

    session_client.ok()
  }

  #[allow(clippy::unused_async)]
  #[tracing::instrument(skip_all)]
  async fn process_command(&mut self, session_set_command: SessionSetCommand) -> Result<(), Error> {
    match session_set_command {
      SessionSetCommand::NewSession {
        sender,
        lean_path,
        lean_server_log_dirpath,
      } => self
        .new_session(&lean_path, lean_server_log_dirpath.as_deref())?
        .send_to_oneshot(sender)?,
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
  async fn run(mut self) -> Result<(), Error> {
    loop {
      tokio::select! {
        session_set_command_opt = self.receiver.recv() => match session_set_command_opt {
          Some(session_set_command) => self.process_command(session_set_command).await?,
          None => return tracing::info!("session set client has been dropped, ending session set run loop...").ok()
        },
        pair_res = self.join_set.join_next() => match pair_res {
          Some(Ok((session_id, result))) => self.cleanup_session(session_id, result),
          Some(Err(join_error)) => tracing::warn!(%join_error, "session run task failed to execute to completion"),
          None => {},
        }
      }
    }
  }

  #[tracing::instrument(skip_all)]
  pub async fn run_session(lean_path: PathBuf, lean_server_log_dirpath: Option<PathBuf>) -> Result<(), Error> {
    let (session_set, session_set_client) = Self::new();
    let session_set_run_task = session_set.run().spawn_task();

    // NOTE:
    // - if [session_set.run()] is not already running, then
    //   [session_set_client.new_session()] will hang, as it is the
    //   [session_set.run()] that processes the issued [NewSession] command
    // - assign the result of [session_set_client.new_session()] to a variable to
    //   prevent it from being immediately dropped and closing the associated
    //   Session object's receiver
    let _session_client = session_set_client
      .new_session(lean_path, lean_server_log_dirpath)
      .await?;

    session_set_run_task.await?
  }
}
