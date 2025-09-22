use std::path::{Path, PathBuf};

use anyhow::Error;
use derive_more::Constructor;
use tokio::{
  sync::{
    mpsc::{Receiver as MpscReceiver, Sender as MpscSender},
    oneshot::Sender as OneshotSender,
  },
  task::JoinHandle,
};
use uuid::Uuid;

use crate::{lean_server::LeanServer, utils::Utils};

pub enum SessionCommand {
  Noop { sender: OneshotSender<()> },
}

#[derive(Clone, Constructor)]
pub struct SessionClient {
  id: Uuid,
  sender: MpscSender<SessionCommand>,
}

impl SessionClient {
  pub fn id(&self) -> Uuid {
    self.id
  }

  // TODO-8dffbb
  #[allow(dead_code)]
  pub async fn noop(&self) -> Result<(), Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let noop_command = SessionCommand::Noop { sender };

    self.sender.send(noop_command).await?;

    receiver.await?.ok()
  }
}

pub struct Session {
  id: Uuid,
  lean_server_run_task: JoinHandle<Result<(), Error>>,
  project_dirpath: PathBuf,
  receiver: MpscReceiver<SessionCommand>,
}

impl Session {
  const COMMAND_CHANNEL_BUFFER_SIZE: usize = 64;
  const MANIFEST_FILE_NAME: &'static str = "lake-manifest.json";

  pub fn new(lean_path: &Path, lean_server_log_dirpath: Option<&Path>) -> Result<(Self, SessionClient), Error> {
    let id = Uuid::new_v4();
    let project_dirpath = Self::project_dirpath(lean_path)?;
    let lean_server_run_task = LeanServer::new(&project_dirpath, lean_server_log_dirpath)?
      .run()
      .spawn_task();
    let (sender, receiver) = tokio::sync::mpsc::channel(Self::COMMAND_CHANNEL_BUFFER_SIZE);
    let session = Self {
      id,
      lean_server_run_task,
      project_dirpath,
      receiver,
    };
    let session_client = SessionClient::new(id, sender);

    tracing::info!(id = %session.id, project_dirpath = %session.project_dirpath.display(), "new session");

    (session, session_client).ok()
  }

  pub fn id(&self) -> Uuid {
    self.id
  }

  fn project_dirpath(lean_path: &Path) -> Result<PathBuf, Error> {
    for ancestor_path in lean_path.ancestors() {
      let mut manifest_filepath = ancestor_path.with_file_name(Self::MANIFEST_FILE_NAME);

      if manifest_filepath.is_file() {
        manifest_filepath.pop();

        return manifest_filepath.ok();
      }
    }

    anyhow::bail!("unable to get project dirpath: no manifest file found in ancestor dirpaths");
  }

  #[allow(clippy::unused_async)]
  #[tracing::instrument(skip_all)]
  async fn process_command(&mut self, session_command: SessionCommand) -> Result<(), Error> {
    match session_command {
      SessionCommand::Noop { sender } => ().send_to_oneshot(sender),
    }
  }

  // TODO-8dffbb
  #[tracing::instrument(skip_all)]
  pub async fn run(mut self) -> Result<(), Error> {
    loop {
      tokio::select! {
        session_command_opt = self.receiver.recv() => match session_command_opt {
          Some(session_command) => self.process_command(session_command).await?,
          None => return tracing::info!("session client has been dropped, ending session run loop...").ok()
        },
        result = &mut self.lean_server_run_task => result??,
      }
    }
  }
}
