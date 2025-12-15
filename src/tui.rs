use std::{collections::HashMap, io::Error as IoError};

use anyhow::Error as AnyhowError;
use crossterm::event::Event as CrosstermEvent;
use derive_more::Constructor;
use futures::{StreamExt, future::Either, stream::FuturesUnordered};
use mkutils::{Output, PointU16, Socket, Terminal, Utils};
use ratatui::{
  Frame,
  text::Text,
  widgets::{Block, Paragraph},
};
use serde_json::Error as SerdeJsonError;
use tokio_stream::wrappers::BroadcastStream as BroadcastReceiverStream;
use ulid::Ulid;

use crate::{commands::TuiCommand, notification::Notification, session_map::SessionMap};

pub type Event = Either<CrosstermEvent, Notification>;

pub struct Tui {
  id: Ulid,
  socket: Socket,
  session_id: Ulid,
  notification_stream: BroadcastReceiverStream<Notification>,
  notifications: Vec<Notification>,
  terminal: Terminal,
}

impl Tui {
  fn new(
    socket: Socket,
    session_id: Ulid,
    size: PointU16,
    notification_stream: BroadcastReceiverStream<Notification>,
  ) -> Result<Self, IoError> {
    let id = Ulid::new();
    let notifications = Vec::new();
    let terminal = Terminal::new(size)?;
    let tui = Self {
      id,
      socket,
      session_id,
      notification_stream,
      notifications,
      terminal,
    };

    tui.ok()
  }

  async fn next_event_impl(&mut self) -> Output<Event, AnyhowError> {
    let event = self
      .socket
      .recv::<CrosstermEvent>()
      .into_select(self.notification_stream.next())
      .await;
    let event = match event {
      Either::Left(crossterm_event_output) => crossterm_event_output?.into_left(),
      Either::Right(notification_res_opt) => notification_res_opt??.into_right(),
    };

    event.into()
  }

  async fn next_event(&mut self) -> TuiEvent {
    TuiEvent::new(self.id, self.next_event_impl().await)
  }

  #[allow(clippy::unused_async)]
  async fn on_crossterm_event(&mut self, crossterm_event: CrosstermEvent) -> Output<(), AnyhowError> {
    match crossterm_event {
      CrosstermEvent::Resize(num_cols, num_rows) => self.terminal.resize(num_cols, num_rows)?.into(),
      CrosstermEvent::Key(_key_event) => Output::EndOk,
      _ignored_event => ().into(),
    }
  }

  pub async fn on_event(&mut self, event: Event) -> Output<(), AnyhowError> {
    match event {
      Either::Left(crossterm_event) => self.on_crossterm_event(crossterm_event).await,
      Either::Right(notification) => self.notifications.push(notification).into(),
    }
  }

  fn render_impl(frame: &mut Frame, session_id: Ulid, notifications: &[Notification]) -> Result<(), AnyhowError> {
    let strings = notifications
      .iter()
      .map(Notification::to_json_str)
      .collect::<Result<Text, SerdeJsonError>>()?;
    let block = Block::bordered().title(session_id.to_string());
    let paragraph = Paragraph::new(strings).block(block);

    frame.render_widget(paragraph, frame.area());

    ().ok()
  }

  async fn render(&mut self) -> Result<(), AnyhowError> {
    self
      .terminal
      .draw(|frame| Self::render_impl(frame, self.session_id, &self.notifications).io_result())?
      .send_to(&mut self.socket)
      .await?
      .ok()
  }
}

#[derive(Constructor)]
pub struct TuiEvent {
  pub id: Ulid,
  pub event: Output<Event, AnyhowError>,
}

#[derive(Default)]
pub struct TuiMap {
  tui_map: HashMap<Ulid, Tui>,
}

impl TuiMap {
  pub fn on_tui_command(
    &mut self,
    session_map: &SessionMap,
    socket: Socket,
    tui_command: &TuiCommand,
  ) -> Result<(), AnyhowError> {
    let session = session_map.get(tui_command.session_id)?;
    let notification_stream = session.notification_stream();
    let tui = Tui::new(socket, session.id(), tui_command.size, notification_stream)?;

    self.tui_map.insert(tui.id, tui);

    ().ok()
  }

  pub async fn on_tui_event(&mut self, tui_event: TuiEvent) -> Result<(), AnyhowError> {
    let output = match tui_event.event {
      Output::Ok(event) => {
        self
          .tui_map
          .get_mut(&tui_event.id)
          .check_present()?
          .on_event(event)
          .await
      }
      Output::EndOk => Output::EndOk,
      Output::EndErr(error) => Output::EndErr(error),
    };

    match output {
      Output::Ok(()) => return ().ok(),
      Output::EndOk => (),
      Output::EndErr(error) => tracing::warn!(?error),
    }

    self.tui_map.remove(&tui_event.id);

    ().ok()
  }

  pub async fn next_event(&mut self) -> TuiEvent {
    self
      .tui_map
      .values_mut()
      .map(Tui::next_event)
      .collect::<FuturesUnordered<_>>()
      .next()
      .wait_then_unwrap_or_pending()
      .await
  }

  pub async fn render(&mut self) -> Result<(), AnyhowError> {
    self.tui_map.values_mut().map(Tui::render).try_join_all().await
  }
}
