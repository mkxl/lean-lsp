use std::collections::HashMap;

use anyhow::Error as AnyhowError;
use crossterm::event::Event;
use derive_more::Constructor;
use mkutils::{Socket, Utils};
use ulid::Ulid;

use crate::{commands::TuiCommand, session_map::SessionMap, tui::Tui};

#[derive(Constructor)]
pub struct TuiEvent {
  pub tui_id: Ulid,
  pub event: Result<Event, AnyhowError>,
}

#[derive(Default)]
pub struct TuiMap {
  tuis: HashMap<Ulid, Tui>,
}

impl TuiMap {
  async fn next_tui_event_impl((tui_id, tui): (&Ulid, &mut Tui)) -> TuiEvent {
    TuiEvent::new(tui_id.copied(), tui.next_event().await)
  }

  pub async fn next_tui_event(&mut self) -> TuiEvent {
    self.tuis.iter_mut().map(Self::next_tui_event_impl).select_all().await
  }

  pub fn on_event(&mut self, tui_event: TuiEvent, session_map: &SessionMap) -> Result<(), AnyhowError> {
    let unit_output = self
      .tuis
      .try_get_mut(tui_event.tui_id)?
      .on_event(tui_event.event?, session_map);
    let Some(unit_res) = unit_output.into_end() else { return ().ok() };

    self.tuis.remove(&tui_event.tui_id);
    unit_res.log_if_error().mem_drop();

    ().ok()
  }

  pub fn on_tui_command(&mut self, socket: Socket, tui_command: &TuiCommand) -> Result<(), AnyhowError> {
    let tui = Tui::new(socket, tui_command)?;

    self.tuis.insert(tui.id(), tui);

    ().ok()
  }

  pub async fn render(&mut self, session_map: &SessionMap) -> Result<(), AnyhowError> {
    self
      .tuis
      .values_mut()
      .stream()
      .then_try_collect(async |tui| tui.render(session_map).await)
      .await
  }
}
