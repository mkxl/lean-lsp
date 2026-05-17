use std::io::Error as IoError;

use anyhow::Error as AnyhowError;
use crossterm::event::Event;
use derive_more::Constructor;
use mkutils::{Socket, Utils};

use crate::{commands::TuiCommand, tui::Tui, widget_set::WidgetSet};

#[derive(Constructor)]
pub struct TuiEvent {
  pub index: usize,
  pub event: Event,
}

#[derive(Default)]
pub struct TuiSet {
  tuis: Vec<Tui>,
}

impl TuiSet {
  pub fn push(&mut self, socket: Socket, tui_command: &TuiCommand) -> Result<(), IoError> {
    let tui = Tui::new(socket, tui_command)?;

    self.tuis.push(tui);

    ().ok()
  }

  pub fn on_tui_event(&mut self, tui_event: TuiEvent, widget_set: &mut WidgetSet) -> Result<(), AnyhowError> {
    let unit_output = self
      .tuis
      .get_mut(tui_event.index)
      .check_present()?
      .on_event(tui_event.event, widget_set);
    let Some(unit_res) = unit_output.into_end() else { return ().ok() };

    self.tuis.remove(tui_event.index);
    unit_res.log_if_error().mem_drop();

    ().ok()
  }

  async fn next_tui_event_impl((index, tui): (usize, &mut Tui)) -> Result<TuiEvent, AnyhowError> {
    TuiEvent::new(index, tui.next_event().await?).ok()
  }

  pub async fn next_tui_event(&mut self) -> Result<TuiEvent, AnyhowError> {
    self
      .tuis
      .iter_mut()
      .enumerate()
      .map(Self::next_tui_event_impl)
      .select_all()
      .await
  }

  pub async fn render(&mut self, widget_set: &mut WidgetSet) -> Result<(), AnyhowError> {
    // NOTE-ff2f17
    for tui in &mut self.tuis {
      tui.render(widget_set).await?;
    }

    ().ok()
  }
}
