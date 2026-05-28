use std::io::Error as IoError;

use anyhow::Error as AnyhowError;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use mkutils::{Output, ScrollCountType, ScrollView, Socket, Terminal, Utils};
use ratatui::{
  Frame,
  layout::{Constraint, Layout, Rect},
};

use crate::{
  commands::TuiCommand,
  widget_set::{View, WidgetSet},
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
  latest_area_set: AreaSet,
}

impl Tui {
  const FORCE_FULL_REDRAWS: bool = false;
  const SCROLL_COUNT: usize = 1;

  pub fn new(socket: Socket, tui_command: &TuiCommand) -> Result<Self, IoError> {
    let terminal = Terminal::new(tui_command.size, Self::FORCE_FULL_REDRAWS)?;
    let latest_area_set = AreaSet::new(Rect::ZERO);
    let tui = Self {
      socket,
      terminal,
      latest_area_set,
    };

    tui.ok()
  }

  pub async fn next_event(&mut self) -> Result<Event, AnyhowError> {
    self.socket.recv().await.into_option().check_next()?
  }

  fn get_active_view<'a>(&self, mouse_event: MouseEvent, widget_set: &'a mut WidgetSet) -> Option<&'a mut View> {
    let position = (mouse_event.column, mouse_event.row).into();

    if self.latest_area_set.goals.contains(position) {
      widget_set.goals_mut().some()
    } else if self.latest_area_set.hover_info.contains(position) {
      widget_set.hover_info_mut().some()
    } else if self.latest_area_set.messages.contains(position) {
      widget_set.messages_mut().some()
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

  fn on_mouse_event(&self, mouse_event: MouseEvent, widget_set: &mut WidgetSet) {
    let Some(active_view) = self.get_active_view(mouse_event, widget_set) else { return };

    let scroll_count_type = if mouse_event.modifiers.intersects(KeyModifiers::CONTROL) {
      ScrollCountType::PageSize
    } else {
      ScrollCountType::Fixed(Self::SCROLL_COUNT)
    };

    match mouse_event.kind {
      MouseEventKind::ScrollDown => active_view.scroll_view_state_mut().scroll_down(scroll_count_type),
      MouseEventKind::ScrollUp => active_view.scroll_view_state_mut().scroll_up(scroll_count_type),
      MouseEventKind::ScrollLeft => active_view.scroll_view_state_mut().scroll_left(scroll_count_type),
      MouseEventKind::ScrollRight => active_view.scroll_view_state_mut().scroll_right(scroll_count_type),
      _ignored_mouse_event_kind => {}
    }
  }

  pub fn on_event(&mut self, event: Event, widget_set: &mut WidgetSet) -> Output<(), AnyhowError> {
    match event {
      Event::Resize(num_cols, num_rows) => self.terminal.resize(num_cols, num_rows)?.output_ok(),
      Event::Key(key_event) => Self::on_key_event(key_event),
      Event::Mouse(mouse_event) => self.on_mouse_event(mouse_event, widget_set).output_ok(),
      _ignored_event => ().output_ok(),
    }
  }

  fn render_impl(
    widget_set: &mut WidgetSet,
    latest_area_set: &mut AreaSet,
    frame: &mut Frame,
  ) -> Result<(), AnyhowError> {
    *latest_area_set = AreaSet::new(frame.area());

    widget_set.goals_mut().render(frame, latest_area_set.goals);
    widget_set.hover_info_mut().render(frame, latest_area_set.hover_info);
    widget_set.messages_mut().render(frame, latest_area_set.messages);

    ().ok()
  }

  pub async fn render(&mut self, widget_set: &mut WidgetSet) -> Result<(), AnyhowError> {
    self
      .terminal
      .draw(|frame| Self::render_impl(widget_set, &mut self.latest_area_set, frame))?
      .take_bytes()
      .send_to(&mut self.socket)
      .await?
      .ok()
  }
}
