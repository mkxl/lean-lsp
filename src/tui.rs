use std::{collections::HashMap, io::Error as IoError, time::Duration};

use anyhow::Error as AnyhowError;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use mkutils::{Constructor, Output, Point, ScrollCountType, ScrollViewState, ScrollWhen, Socket, Terminal, Utils};
use ratatui::{Frame, layout::Rect};
use ulid::Ulid;

use crate::{commands::TuiCommand, info_view::InfoView, session_map::SessionMap};

#[derive(Default)]
struct AreaSet;

impl AreaSet {
  const fn new(_area: Rect) -> Self {
    Self
  }
}

#[derive(Constructor)]
#[constructor(from_values)]
struct RenderState {
  scroll_view_state: ScrollViewState,
}

impl RenderState {
  const DEFAULT_SCROLL_WHEN: ScrollWhen = ScrollWhen::ForLargeContent;
  const DEFAULT_RENDER_SCROLL_BARS_TIMEOUT: Option<Duration> = Some(Duration::from_secs(1));
  const DEFAULT_SCROLL_VIEW_STATE: ScrollViewState =
    ScrollViewState::new(Self::DEFAULT_SCROLL_WHEN, Self::DEFAULT_RENDER_SCROLL_BARS_TIMEOUT);

  const fn new() -> Self {
    Self::from_values(Self::DEFAULT_SCROLL_VIEW_STATE)
  }

  fn scroll_view_state_mut(&mut self) -> &mut ScrollViewState {
    self.scroll_view_state.ref_mut()
  }
}

impl Default for RenderState {
  fn default() -> Self {
    Self::new()
  }
}

struct Render;

impl Render {
  const fn new(_info_view: &InfoView, _info_view_state: &mut RenderState) -> Self {
    Self
  }

  #[expect(clippy::unused_self)]
  const fn render(&self, _frame: &mut Frame) {
    // std::todo!()
  }
}

#[derive(Default)]
struct TuiState {
  latest_area_set: AreaSet,
  render_state_map: HashMap<Ulid, RenderState>,
  active_session_id: Option<Ulid>,
}

impl TuiState {
  fn sync(&mut self, session_map: &SessionMap) -> Option<(Ulid, &mut RenderState)> {
    self
      .render_state_map
      .retain(|session_id, _info_view_state| session_map.contains(session_id));

    let new_active_session_id = if let Some(active_session_id) = self.active_session_id
      && session_map.contains(&active_session_id)
    {
      active_session_id
    } else {
      session_map.random_session_id()?
    };
    let render_state = self.render_state_map.entry(new_active_session_id).or_default();

    self.active_session_id = new_active_session_id.some();

    new_active_session_id.pair(render_state).some()
  }

  const fn render_empty(_frame: &mut Frame) {
    // std::todo!()
  }

  fn render(&mut self, session_map: &SessionMap, frame: &mut Frame) -> Result<(), AnyhowError> {
    self.latest_area_set = AreaSet::new(frame.area());

    let Some((active_session_id, render_state)) = self.sync(session_map) else {
      return Self::render_empty(frame).ok();
    };
    let info_view = session_map.try_get(active_session_id)?.info_view();
    let render = Render::new(info_view, render_state);

    render.render(frame);

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
    let Some((_active_session_id, render_state)) = self.tui_state.sync(session_map) else { return };
    let scroll_count_type = mouse_event
      .modifiers
      .pipe_into(Self::get_scoll_count_type)
      .pipe_into(Point::from_scalar);

    render_state
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
