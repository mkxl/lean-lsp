use std::io::Error as IoError;

use anyhow::Error as AnyhowError;
use crossterm::event::Event;
use mkutils::{Output, Socket, Terminal, Utils};
use ratatui::{
  Frame,
  layout::{Constraint, Layout, Rect},
  prelude::Stylize,
  style::{Style, Styled},
  text::Line,
  widgets::{Block, Paragraph},
};
use ulid::Ulid;

use crate::{
  commands::TuiCommand,
  open_file::OpenFileMap,
  render_state::RenderState,
  types::{Position, Severity, Utf16},
};

pub struct Tui {
  socket: Socket,
  terminal: Terminal,
}

impl Tui {
  const CONSTRAINTS: [Constraint; 3] = [Constraint::Fill(2), Constraint::Fill(1), Constraint::Fill(2)];
  const MESSAGE_NO_GOALS: &str = "No goals";
  const MESSAGE_NO_MESSAGE: &str = "No messages";
  const MESSAGE_NO_HOVER: &str = "No information available";
  const STYLE_BLOCK_TITLE: Style = Style::new().dark_gray();
  const STYLE_BLOCK_BORDER: Style = Style::new().white().bold();
  const TITLE_GOALS: &str = "Goals";
  const TITLE_MESSAGES: &str = "Messages";
  const TITLE_HOVER: &str = "Hover Info";
  const TURNSTILE: &str = "⊢ ";

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

  fn block(title: &str) -> Block<'_> {
    let title = std::format!(" {title} ")
      .convert::<Line>()
      .centered()
      .set_style(Self::STYLE_BLOCK_TITLE);

    Block::bordered().border_style(Self::STYLE_BLOCK_BORDER).title(title)
  }

  fn hypothesis_line(line: &str) -> Line<'_> {
    let Some((prefix, colon, suffix)) = line.find_substr(" : ") else { return line.white().into() };

    prefix.green().add_span(colon.dark_gray()).add_span(suffix.white())
  }

  fn goal_line(line: &str) -> Line<'_> {
    if let Some(suffix) = line.strip_prefix(Self::TURNSTILE) {
      Self::TURNSTILE.blue().bold().add_span(suffix.cyan())
    } else {
      Self::hypothesis_line(line)
    }
  }

  fn render_goals_impl(render_state: &RenderState) -> Vec<Line<'_>> {
    let Some(plain_goals) = &render_state.plain_goals else {
      return Self::MESSAGE_NO_GOALS.dim().convert::<Line>().singleton();
    };

    if plain_goals.goals.is_empty() {
      return Self::MESSAGE_NO_GOALS.dim().convert::<Line>().singleton();
    }

    let mut lines = Vec::new();
    let multiple_goals = 1 < plain_goals.goals.len();

    for (index, goal_str) in plain_goals.goals.iter().enumerate() {
      if index.is_positive() {
        lines.push(Line::default());
      }

      if multiple_goals {
        let case_num = index.incremented();

        std::format!("case {case_num}")
          .yellow()
          .bold()
          .convert::<Line>()
          .push_to(&mut lines);
      }

      goal_str.lines().map(Self::goal_line).push_all_to(&mut lines);
    }

    lines
  }

  fn render_goals(frame: &mut Frame, session_id: Ulid, render_state: &RenderState, area: Rect) {
    let title = std::format!("{title} [{session_id}]", title = Self::TITLE_GOALS);
    let block = Self::block(&title);
    let lines = Self::render_goals_impl(render_state);

    Paragraph::new(lines).block(block).render_to(frame, area);
  }

  fn render_hover_impl(render_state: &RenderState) -> Vec<Line<'_>> {
    if let Some(hover_file_result) = &render_state.hover_file_result {
      hover_file_result.contents.value.lines().map(Self::goal_line).collect()
    } else {
      Self::MESSAGE_NO_HOVER.dim().convert::<Line>().singleton()
    }
  }

  fn render_hover(frame: &mut Frame, render_state: &RenderState, area: Rect) {
    let block = Self::block(Self::TITLE_HOVER);
    let lines = Self::render_hover_impl(render_state);

    Paragraph::new(lines).block(block).render_to(frame, area);
  }

  fn push_message<'a>(
    lines: &mut Vec<Line<'a>>,
    file_name: &'a str,
    position: Position<Utf16>,
    severity: Severity,
    message: &'a str,
  ) {
    let file_name = file_name.white().bold();
    let row = position.line.incremented();
    let col = position.character.incremented();
    let suffix = std::format!(":{row}:{col}").dark_gray();
    let color = severity.color();
    let label = severity.label().bold().fg(color);
    let line_1 = file_name.add_span(suffix).add_span(" ").add_span(label);
    let line_2 = "  ".add_span(message).fg(color);
    let line_3 = Line::default();

    lines.push(line_1);
    lines.push(line_2);
    lines.push(line_3);
  }

  fn render_messages(frame: &mut Frame, render_state: &RenderState, area: Rect) {
    let block = Self::block(Self::TITLE_MESSAGES);
    let mut lines = Vec::new();
    let mut rendered_message = false;

    for (text_document, file_state) in &render_state.file_states {
      let file_name = text_document.file_name();

      for processing in &file_state.processing {
        Self::push_message(
          &mut lines,
          file_name,
          processing.range.start,
          processing.kind.severity(),
          processing.kind.message(),
        );
        rendered_message.set_true();
      }

      for diagnostic in &file_state.diagnostics {
        Self::push_message(
          &mut lines,
          file_name,
          diagnostic.range.start,
          diagnostic.severity,
          &diagnostic.message,
        );
        rendered_message.set_true();
      }
    }

    if !rendered_message {
      Self::MESSAGE_NO_MESSAGE.dim().convert::<Line>().push_to(&mut lines);
    }

    Paragraph::new(lines).block(block).render_to(frame, area);
  }

  fn render_impl(
    frame: &mut Frame,
    session_id: Ulid,
    render_state: &RenderState,
    _open_files: &OpenFileMap,
  ) -> Result<(), AnyhowError> {
    let layout = Layout::vertical(Self::CONSTRAINTS);
    let [goals_area, hover_area, messages_area] = layout.areas(frame.area());

    Self::render_goals(frame, session_id, render_state, goals_area);
    Self::render_hover(frame, render_state, hover_area);
    Self::render_messages(frame, render_state, messages_area);

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
