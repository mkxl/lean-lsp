use std::collections::HashMap;

use anyhow::Error as AnyhowError;
use mkutils::{Event, Socket, Utils};
use ulid::Ulid;

use crate::{
  commands::{
    ChangeFileCommand, CloseFileCommand, FileCommand, GetCommand, InfoViewCommand, KillCommand, NewSessionCommand,
    NotificationsCommand, OpenFileCommand, RebuildCommand,
  },
  session::{Session, SessionMessage},
  types::{AppError, SessionInfo},
};

#[derive(Default)]
pub struct SessionMap {
  sessions: HashMap<Ulid, Session>,
}

impl SessionMap {
  pub fn random_session_id(&self) -> Option<Ulid> {
    self.sessions.keys().next()?.copied().some()
  }

  pub fn contains(&self, session_id: &Ulid) -> bool {
    self.sessions.contains_key(session_id)
  }

  pub fn try_get(&self, session_id: impl Into<Option<Ulid>>) -> Result<&Session, AnyhowError> {
    self.sessions.try_get(session_id)
  }

  pub async fn next_message(&mut self) -> SessionMessage {
    self.sessions.values_mut().map(Session::next_message).select_all().await
  }

  pub async fn send_keep_alive(&mut self) -> Result<(), AnyhowError> {
    self
      .sessions
      .values_mut()
      .map(Session::send_keep_alive)
      .try_join_all_into()
      .await
  }

  pub async fn on_file_command(&mut self, socket: Socket, file_command: FileCommand) -> Result<(), AnyhowError> {
    match file_command {
      FileCommand::Change(change_file_command) => {
        self
          .sessions
          .try_get_mut(change_file_command.session_id)?
          .change_file(&change_file_command)
          .await
          .respond_to::<ChangeFileCommand>(socket)
          .await
      }
      FileCommand::Close(close_file_command) => {
        self
          .sessions
          .try_get_mut(close_file_command.session_id)?
          .close_file(&close_file_command)
          .await
          .respond_to::<CloseFileCommand>(socket)
          .await
      }
      FileCommand::Hover(hover_file_command) => {
        self
          .sessions
          .try_get_mut(hover_file_command.session_id)?
          .hover_file(socket, &hover_file_command)
          .await
      }
      FileCommand::Open(open_file_command) => {
        self
          .sessions
          .try_get_mut(open_file_command.session_id)?
          .open_file(open_file_command)
          .await
          .respond_to::<OpenFileCommand>(socket)
          .await
      }
    }
  }

  pub fn on_get_command(&self, get_command: &GetCommand) -> Result<SessionInfo, AppError> {
    self.sessions.try_get(get_command.session_id)?.info().ok()
  }

  pub async fn on_info_view_command(
    &mut self,
    socket: Socket,
    info_view_command: &InfoViewCommand,
  ) -> Result<(), AnyhowError> {
    match &info_view_command {
      InfoViewCommand::GetPlainGoals(get_plain_goals_command) => {
        self
          .sessions
          .try_get_mut(get_plain_goals_command.session_id)?
          .get_plain_goals(socket, get_plain_goals_command)
          .await
      }
    }?
    .ok()
  }

  pub async fn on_kill_command(
    &mut self,
    kill_server_event: &Event,
    kill_command: &KillCommand,
  ) -> Result<(), AppError> {
    if let Some(session_id) = kill_command.session_id {
      self.sessions.remove(&session_id).check_present()?.kill().await?.ok()
    } else {
      kill_server_event.set();

      self.sessions.values_mut().map(Session::kill).try_join_all_into().await
    }
  }

  pub fn on_list_command(&self) -> Vec<SessionInfo> {
    self.sessions.values().map(Session::info).collect()
  }

  pub async fn on_new_session_command(
    &mut self,
    socket: Socket,
    new_session_command: NewSessionCommand,
  ) -> Result<(), AnyhowError> {
    let mut session = match Session::new(new_session_command) {
      Ok(session) => session,
      Err(app_error) => return app_error.err().respond_to::<NewSessionCommand>(socket).await,
    };

    // NOTE: if [session.initialize()] returns an error we are unable to send the
    // error over the socket as the socket has already been consumed
    session.initialize(socket).await?;
    self.sessions.insert(session.id(), session);

    ().ok()
  }

  pub fn on_notifications_command(
    &mut self,
    socket: Socket,
    notifications_command: NotificationsCommand,
  ) -> Result<(), AnyhowError> {
    self
      .sessions
      .try_get_mut(notifications_command.session_id)?
      .notify(socket, notifications_command)
      .ok()
  }

  pub async fn on_rebuild_command(
    &mut self,
    socket: Socket,
    rebuild_command: &RebuildCommand,
  ) -> Result<(), AnyhowError> {
    self
      .sessions
      .try_get_mut(rebuild_command.session_id)?
      .rebuild(socket)
      .await
  }

  pub async fn on_message(&mut self, session_message: SessionMessage) -> Result<(), AnyhowError> {
    self
      .sessions
      .try_get_mut(session_message.session_id)?
      .on_message(session_message.message?)
      .await
  }
}
