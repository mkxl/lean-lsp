use std::io::Error as IoError;

use anyhow::Error as AnyhowError;
use crossterm::event::Event;
use derive_more::Constructor;
use mkutils::{Output, Socket, Utils};
use ulid::Ulid;

use crate::{commands::TuiCommand, open_file::OpenFileMap, render_state::RenderState, tui::Tui};

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

  pub async fn on_tui_event(&mut self, tui_event: TuiEvent) -> Result<(), AnyhowError> {
    let unit_output = self
      .tuis
      .get_mut(tui_event.index)
      .check_present()?
      .on_event(tui_event.event)
      .await;

    match unit_output {
      Output::Ok(()) => return ().ok(),
      Output::EndOk => (),
      Output::EndErr(error) => error.log_error(),
    }

    self.tuis.remove(tui_event.index);

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

  pub async fn render(
    &mut self,
    session_id: Ulid,
    render_state: &RenderState,
    open_files: &OpenFileMap,
  ) -> Result<(), AnyhowError> {
    self
      .tuis
      .iter_mut()
      .map(|tui| tui.render(session_id, render_state, open_files))
      .try_join_all()
      .await
  }
}
