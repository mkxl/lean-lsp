use std::io::Error as IoError;

use anyhow::Error as AnyhowError;
use crossterm::event::Event;
use mkutils::{Output, Socket, Terminal, Utils};
use ratatui::Frame;
use ulid::Ulid;

use crate::{commands::TuiCommand, open_file::OpenFileMap, render_state::RenderState};

pub struct Tui {
  socket: Socket,
  terminal: Terminal,
}

impl Tui {
  pub fn new(socket: Socket, tui_command: &TuiCommand) -> Result<Self, IoError> {
    let terminal = Terminal::new(tui_command.size)?;
    let tui = Self { socket, terminal };

    tui.ok()
  }

  pub async fn next_event(&mut self) -> Result<Event, AnyhowError> {
    self.socket.recv().await.into_option().check_next()?
  }

  #[allow(clippy::unused_async)]
  pub async fn on_event(&mut self, event: Event) -> Output<(), AnyhowError> {
    match event {
      Event::Resize(num_cols, num_rows) => self.terminal.resize(num_cols, num_rows)?.into(),
      Event::Key(_key_event) => Output::EndOk,
      _ignored_event => ().into(),
    }
  }

  fn render_impl(
    _frame: &mut Frame,
    _session_id: Ulid,
    _render_state: &RenderState,
    _open_files: &OpenFileMap,
  ) -> Result<(), AnyhowError> {
    ().ok()
  }

  pub async fn render(
    &mut self,
    session_id: Ulid,
    render_state: &RenderState,
    open_files: &OpenFileMap,
  ) -> Result<(), AnyhowError> {
    self
      .terminal
      .draw(|frame| Self::render_impl(frame, session_id, render_state, open_files))?
      .take_byte_str()
      .send_to(&mut self.socket)
      .await?
      .ok()
  }
}
