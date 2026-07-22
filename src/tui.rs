use std::{collections::HashMap, io::Error as IoError};

use anyhow::Error as AnyhowError;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use mkutils::{Output, Point, ScrollCountType, ScrollViewState, Socket, Terminal, Utils};
use ratatui::{Frame, layout::Rect};
use ulid::Ulid;

use crate::{commands::TuiCommand, info_view_content::InfoViewContent, session_map::SessionMap};

#[derive(Default)]
struct AreaSet;

impl AreaSet {
  const fn new(_area: Rect) -> Self {
    Self
  }
}

#[derive(Default)]
struct InfoViewState;

impl InfoViewState {
  #[expect(clippy::needless_pass_by_ref_mut)]
  fn scroll_view_state_mut(&mut self) -> &mut ScrollViewState {
    std::todo!()
  }
}

struct InfoView;

impl InfoView {
  const fn new(_info_view_content: &InfoViewContent, _info_view_state: &mut InfoViewState) -> Self {
    Self
  }

  #[expect(clippy::unused_self)]
  const fn render(&self, _frame: &mut Frame) {}
}

#[expect(clippy::zero_sized_map_values)]
#[derive(Default)]
struct TuiState {
  latest_area_set: AreaSet,
  info_view_state_map: HashMap<Ulid, InfoViewState>,
  active_session_id: Option<Ulid>,
}

impl TuiState {
  fn sync(&mut self, session_map: &SessionMap) -> Option<(Ulid, &mut InfoViewState)> {
    self
      .info_view_state_map
      .retain(|session_id, _info_view_state| session_map.contains(session_id));

    let new_active_session_id = if let Some(active_session_id) = self.active_session_id
      && session_map.contains(&active_session_id)
    {
      active_session_id
    } else {
      session_map.random_session_id()?
    };
    let info_view_state = self.info_view_state_map.entry(new_active_session_id).or_default();

    self.active_session_id = new_active_session_id.some();

    new_active_session_id.pair(info_view_state).some()
  }

  fn render_empty(_frame: &mut Frame) {
    std::todo!()
  }

  fn render(&mut self, session_map: &SessionMap, frame: &mut Frame) -> Result<(), AnyhowError> {
    self.latest_area_set = AreaSet::new(frame.area());

    let Some((active_session_id, info_view_state)) = self.sync(session_map) else {
      return Self::render_empty(frame).ok();
    };
    let info_view_content = session_map.try_get(active_session_id)?.info_view_content();
    let info_view = InfoView::new(info_view_content, info_view_state);

    info_view.render(frame);

    ().ok()
  }
}

#[expect(clippy::struct_field_names)]
pub struct Tui {
  id: Ulid,
  socket: Socket,
  terminal: Terminal,
  tui_state: TuiState,
}

impl Tui {
  const SCROLL_COUNT: usize = 1;
  const TERMINAL_FORCE_FULL_REDRAWS: bool = false;

  pub fn new(socket: Socket, tui_command: &TuiCommand) -> Result<Self, IoError> {
    let id = Ulid::new();
    let terminal = Terminal::new(tui_command.size, Self::TERMINAL_FORCE_FULL_REDRAWS)?;
    let tui_state = TuiState::default();
    let tui = Self {
      id,
      socket,
      terminal,
      tui_state,
    };

    tui.ok()
  }

  pub const fn id(&self) -> Ulid {
    self.id
  }

  pub async fn next_event(&mut self) -> Result<Event, AnyhowError> {
    self.socket.recv().await.into_option().check_next()?
  }

  fn on_key_event(key_event: KeyEvent) -> Output<(), AnyhowError> {
    if key_event.code == KeyCode::Char('c') && key_event.modifiers.intersects(KeyModifiers::CONTROL) {
      Output::EndOk
    } else {
      ().output_ok()
    }
  }

  const fn get_scoll_count_type(mouse_event_modifiers: KeyModifiers) -> ScrollCountType {
    if mouse_event_modifiers.intersects(KeyModifiers::CONTROL) {
      ScrollCountType::PageSize
    } else {
      ScrollCountType::Fixed(Self::SCROLL_COUNT)
    }
  }

  fn on_mouse_event(&mut self, mouse_event: MouseEvent, session_map: &SessionMap) {
    let Some((_active_session_id, info_view_state)) = self.tui_state.sync(session_map) else { return };
    let scroll_count_type = mouse_event
      .modifiers
      .pipe_into(Self::get_scoll_count_type)
      .pipe_into(Point::from_scalar);

    info_view_state
      .scroll_view_state_mut()
      .on_scroll(mouse_event.kind, scroll_count_type);
  }

  pub fn on_event(&mut self, event: Event, session_map: &SessionMap) -> Output<(), AnyhowError> {
    match event {
      Event::Resize(num_cols, num_rows) => self.terminal.resize(num_cols, num_rows)?.output_ok(),
      Event::Key(key_event) => Self::on_key_event(key_event),
      Event::Mouse(mouse_event) => self.on_mouse_event(mouse_event, session_map).output_ok(),
      _ignored_event => ().output_ok(),
    }
  }

  pub async fn render(&mut self, session_map: &SessionMap) -> Result<(), AnyhowError> {
    self
      .terminal
      .draw(|frame| self.tui_state.render(session_map, frame))?
      .take_bytes()
      .send_to(self.socket.ref_mut())
      .await?
      .ok()
  }
}
