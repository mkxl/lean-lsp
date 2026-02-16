use std::io::Error as IoError;

use anyhow::Error as AnyhowError;
use crossterm::event::Event;
use mkutils::{Output, Socket, Terminal, Utils};
use ratatui::{
  Frame,
  layout::{Constraint, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{Block, Borders, Padding, Paragraph, Wrap},
};
use ulid::Ulid;

use crate::{commands::TuiCommand, open_file::OpenFileMap, render_state::RenderState, types::Severity};

pub struct Tui {
  socket: Socket,
  terminal: Terminal,
}

impl Tui {
  const BORDER_STYLE: Style = Style::new().fg(Color::DarkGray);
  const SECTION_TITLE_STYLE: Style = Style::new().fg(Color::White).add_modifier(Modifier::BOLD);
  const TURNSTILE: &str = "⊢ ";
  const NO_GOALS_MESSAGE: &str = "No goals";
  const NO_INFO_MESSAGE: &str = "No info available";

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

  fn render_goals_section(frame: &mut Frame, area: Rect, render_state: &RenderState) {
    let block = Block::new()
      .borders(Borders::ALL)
      .border_style(Self::BORDER_STYLE)
      .title(Line::from(Span::styled(" Tactic State ", Self::SECTION_TITLE_STYLE)))
      .padding(Padding::horizontal(1));

    let Some(plain_goals) = render_state.plain_goals() else {
      let paragraph = Paragraph::new(Text::styled(Self::NO_GOALS_MESSAGE, Style::new().dim()))
        .block(block)
        .wrap(Wrap { trim: false });

      frame.render_widget(paragraph, area);

      return;
    };

    if plain_goals.goals.is_empty() {
      let paragraph = Paragraph::new(Text::styled(Self::NO_GOALS_MESSAGE, Style::new().dim()))
        .block(block)
        .wrap(Wrap { trim: false });

      frame.render_widget(paragraph, area);

      return;
    }

    let mut lines = Vec::new();

    for (goal_index, goal_str) in plain_goals.goals.iter().enumerate() {
      if goal_index > 0 {
        lines.push(Line::raw(""));
      }

      if plain_goals.goals.len() > 1 {
        let header = format!("case {}", goal_index + 1);

        lines.push(Line::styled(header, Style::new().fg(Color::Yellow).bold()));
      }

      for goal_line in goal_str.lines() {
        if let Some(target) = goal_line.strip_prefix(Self::TURNSTILE) {
          lines.push(Line::from(vec![
            Span::styled(Self::TURNSTILE, Style::new().fg(Color::Blue).bold()),
            Span::styled(target, Style::new().fg(Color::Cyan)),
          ]));
        } else {
          lines.push(Self::render_hypothesis_line(goal_line));
        }
      }
    }

    let paragraph = Paragraph::new(Text::from(lines)).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
  }

  fn render_hypothesis_line(line: &str) -> Line<'_> {
    let Some(colon_index) = line.find(" : ") else {
      return Line::styled(line, Style::new().fg(Color::White));
    };

    let (name, rest) = line.split_at(colon_index);

    Line::from(vec![
      Span::styled(name, Style::new().fg(Color::Green)),
      Span::styled(" : ", Style::new().fg(Color::DarkGray)),
      Span::styled(&rest[3..], Style::new().fg(Color::White)),
    ])
  }

  fn render_hover_section(frame: &mut Frame, area: Rect, render_state: &RenderState) {
    let block = Block::new()
      .borders(Borders::ALL)
      .border_style(Self::BORDER_STYLE)
      .title(Line::from(Span::styled(" Expected Type ", Self::SECTION_TITLE_STYLE)))
      .padding(Padding::horizontal(1));

    let Some(hover) = render_state.hover() else {
      let paragraph = Paragraph::new(Text::styled(Self::NO_INFO_MESSAGE, Style::new().dim()))
        .block(block)
        .wrap(Wrap { trim: false });

      frame.render_widget(paragraph, area);

      return;
    };

    let mut lines = Vec::new();

    for hover_line in hover.contents.value.lines() {
      if let Some(target) = hover_line.strip_prefix(Self::TURNSTILE) {
        lines.push(Line::from(vec![
          Span::styled(Self::TURNSTILE, Style::new().fg(Color::Blue).bold()),
          Span::styled(target.to_owned(), Style::new().fg(Color::Cyan)),
        ]));
      } else {
        lines.push(Self::render_hypothesis_line(hover_line));
      }
    }

    let paragraph = Paragraph::new(Text::from(lines)).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
  }

  const fn severity_style(severity: &Severity) -> Style {
    match severity {
      Severity::Error => Style::new().fg(Color::Red),
      Severity::Warning => Style::new().fg(Color::Yellow),
      Severity::Information => Style::new().fg(Color::Blue),
      Severity::Hint => Style::new().fg(Color::DarkGray),
    }
  }

  fn render_messages_section(frame: &mut Frame, area: Rect, render_state: &RenderState) {
    let block = Block::new()
      .borders(Borders::ALL)
      .border_style(Self::BORDER_STYLE)
      .title(Line::from(Span::styled(" All Messages ", Self::SECTION_TITLE_STYLE)))
      .padding(Padding::horizontal(1));

    let mut lines = Vec::new();
    let mut has_diagnostics = false;

    for (filepath, file_state) in render_state.file_states() {
      let filename = filepath
        .file_name()
        .unwrap_or_else(|| filepath.as_str());

      if file_state.is_processing {
        lines.push(Line::from(vec![
          Span::styled(filename, Style::new().fg(Color::White).bold()),
          Span::styled(" [processing]", Style::new().fg(Color::Yellow).italic()),
        ]));
      }

      for diagnostic in &file_state.diagnostics {
        has_diagnostics = true;

        let severity_style = Self::severity_style(&diagnostic.severity);
        let label = diagnostic.severity.label();
        let line_num = diagnostic.range.start().line + 1;
        let col_num = diagnostic.range.start().character + 1;

        lines.push(Line::from(vec![
          Span::styled(filename, Style::new().fg(Color::White).bold()),
          Span::styled(format!(":{line_num}:{col_num}"), Style::new().fg(Color::DarkGray)),
          Span::raw(" "),
          Span::styled(label, severity_style.add_modifier(Modifier::BOLD)),
        ]));

        for message_line in diagnostic.message.lines() {
          lines.push(Line::styled(
            format!("  {message_line}"),
            severity_style,
          ));
        }

        lines.push(Line::raw(""));
      }
    }

    if !has_diagnostics {
      lines.push(Line::styled("No messages", Style::new().dim()));
    }

    let paragraph = Paragraph::new(Text::from(lines)).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
  }

  fn render_header(frame: &mut Frame, area: Rect, session_id: Ulid) {
    let title = format!(" Lean Info View ─ {session_id} ");
    let line = Line::styled(title, Style::new().fg(Color::Cyan).bold()).centered();
    let paragraph = Paragraph::new(line);

    frame.render_widget(paragraph, area);
  }

  fn render_impl(
    frame: &mut Frame,
    session_id: Ulid,
    render_state: &RenderState,
    _open_files: &OpenFileMap,
  ) -> Result<(), AnyhowError> {
    let area = frame.area();

    let [header_area, body_area] = Layout::vertical([
      Constraint::Length(1),
      Constraint::Fill(1),
    ])
    .areas(area);

    Self::render_header(frame, header_area, session_id);

    let [goals_area, hover_area, messages_area] = Layout::vertical([
      Constraint::Fill(2),
      Constraint::Fill(1),
      Constraint::Fill(2),
    ])
    .areas(body_area);

    Self::render_goals_section(frame, goals_area, render_state);
    Self::render_hover_section(frame, hover_area, render_state);
    Self::render_messages_section(frame, messages_area, render_state);

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
