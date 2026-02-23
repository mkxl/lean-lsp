use std::io::Error as IoError;

use anyhow::Error as AnyhowError;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use mkutils::{Output, ScrollCountType, ScrollViewState, ScrollWhen, Socket, Terminal, Utils};
use ratatui::{
  Frame,
  layout::{Constraint, Layout, Rect},
};

use crate::{
  commands::TuiCommand,
  widget_set::{WidgetSet, WidgetStateSet},
};

struct AreaSet {
  goals: Rect,
  hover_info: Rect,
  messages: Rect,
}

impl AreaSet {
  const CONSTRAINTS: [Constraint; 3] = [Constraint::Fill(2), Constraint::Fill(1), Constraint::Fill(2)];

  fn new(area: Rect) -> Self {
    let layout = Layout::vertical(Self::CONSTRAINTS);
    let [goals, hover_info, messages] = layout.areas(area);

    Self {
      goals,
      hover_info,
      messages,
    }
  }
}

pub struct Tui {
  socket: Socket,
  terminal: Terminal,
  widget_state_set: WidgetStateSet,
  latest_area_set: AreaSet,
}

impl Tui {
  const SCROLL_COUNT: usize = 1;
  const SCROLL_WHEN: ScrollWhen = ScrollWhen::ForLargeContent;

  pub fn new(socket: Socket, tui_command: &TuiCommand) -> Result<Self, IoError> {
    let terminal = Terminal::new(tui_command.size)?;
    let widget_state_set = WidgetStateSet::new(Self::SCROLL_WHEN);
    let latest_area_set = AreaSet::new(Rect::ZERO);
    let tui = Self {
      socket,
      terminal,
      widget_state_set,
      latest_area_set,
    };

    tui.ok()
  }

  pub async fn next_event(&mut self) -> Result<Event, AnyhowError> {
    self.socket.recv().await.into_option().check_next()?
  }

  fn active_scroll_view_state(&mut self, mouse_event: MouseEvent) -> Option<&mut ScrollViewState> {
    let position = (mouse_event.column, mouse_event.row).into();

    if self.latest_area_set.goals.contains(position) {
      self.widget_state_set.goals_mut().some()
    } else if self.latest_area_set.hover_info.contains(position) {
      self.widget_state_set.hover_info_mut().some()
    } else if self.latest_area_set.messages.contains(position) {
      self.widget_state_set.messages_mut().some()
    } else {
      None
    }
  }

  fn on_key_event(key_event: KeyEvent) -> Output<(), AnyhowError> {
    if key_event.code == KeyCode::Char('c') && key_event.modifiers.intersects(KeyModifiers::CONTROL) {
      Output::EndOk
    } else {
      ().output_ok()
    }
  }

  fn on_mouse_event(&mut self, mouse_event: MouseEvent) {
    let Some(scroll_view_state) = self.active_scroll_view_state(mouse_event) else { return };

    let scroll_count_type = if mouse_event.modifiers.intersects(KeyModifiers::CONTROL) {
      ScrollCountType::PageSize
    } else {
      ScrollCountType::Fixed(Self::SCROLL_COUNT)
    };

    match mouse_event.kind {
      MouseEventKind::ScrollDown => scroll_view_state.scroll_down(scroll_count_type),
      MouseEventKind::ScrollUp => scroll_view_state.scroll_up(scroll_count_type),
      MouseEventKind::ScrollLeft => scroll_view_state.scroll_left(scroll_count_type),
      MouseEventKind::ScrollRight => scroll_view_state.scroll_right(scroll_count_type),
      _ignored_mouse_event_kind => {}
    }
  }

  pub fn on_event(&mut self, event: Event) -> Output<(), AnyhowError> {
    match event {
      Event::Resize(num_cols, num_rows) => self.terminal.resize(num_cols, num_rows)?.output_ok(),
      Event::Key(key_event) => Self::on_key_event(key_event),
      Event::Mouse(mouse_event) => self.on_mouse_event(mouse_event).output_ok(),
      _ignored_event => ().output_ok(),
    }
  }

  fn render_impl(
    widget_set: &WidgetSet,
    latest_area_set: &mut AreaSet,
    widget_state_set: &mut WidgetStateSet,
    frame: &mut Frame,
  ) -> Result<(), AnyhowError> {
    *latest_area_set = AreaSet::new(frame.area());

    widget_set
      .goals()
      .render_with_state(frame, latest_area_set.goals, widget_state_set.goals_mut());
    widget_set
      .hover_info()
      .render_with_state(frame, latest_area_set.hover_info, widget_state_set.hover_info_mut());
    widget_set
      .messages()
      .render_with_state(frame, latest_area_set.messages, widget_state_set.messages_mut());

    ().ok()
  }

  pub async fn render(&mut self, widget_set: &WidgetSet) -> Result<(), AnyhowError> {
    self
      .terminal
      .draw(|frame| Self::render_impl(widget_set, &mut self.latest_area_set, &mut self.widget_state_set, frame))?
      .take_byte_str()
      .send_to(&mut self.socket)
      .await?
      .ok()
  }
}
