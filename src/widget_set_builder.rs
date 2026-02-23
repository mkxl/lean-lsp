use std::collections::HashMap;

use getset::{Getters, MutGetters};
use mkutils::Utils;

use crate::{
  notification::{FileProgress, Notification, PublishDiagnostics},
  responses::{GetPlainGoalsResponse, HoverFileResponse},
  types::{Diagnostic, HoverFileResult, PlainGoals, Processing, TextDocument},
  widget_set::WidgetSet,
};

#[derive(Default, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub struct FileState {
  diagnostics: Vec<Diagnostic>,
  processing: Vec<Processing>,
}

#[derive(Getters, MutGetters)]
pub struct WidgetSetBuilder {
  #[get = "pub"]
  file_states: HashMap<TextDocument, FileState>,

  #[get = "pub"]
  plain_goals: Option<PlainGoals>,

  #[get = "pub"]
  hover_file_result: Option<HoverFileResult>,
}

impl WidgetSetBuilder {
  pub fn new() -> Self {
    let file_states = HashMap::new();
    let plain_goals = None;
    let hover_file_result = None;

    Self {
      file_states,
      plain_goals,
      hover_file_result,
    }
  }

  pub fn build(&self) -> WidgetSet {
    WidgetSet::new(self)
  }

  fn update_file_states<T>(
    &mut self,
    text_document: TextDocument,
    getter: impl FnOnce(&mut FileState) -> &mut T,
    new_value: T,
  ) -> WidgetSet {
    self
      .file_states
      .entry(text_document)
      .or_default()
      .pipe_into(getter)
      .mem_replace(new_value)
      .mem_drop();

    self.build()
  }

  fn on_file_progress(&mut self, file_progress: FileProgress) -> WidgetSet {
    self.update_file_states(
      file_progress.text_document,
      FileState::processing_mut,
      file_progress.processing,
    )
  }

  fn on_publish_diagnostics(&mut self, publish_diagnostics: PublishDiagnostics) -> WidgetSet {
    self.update_file_states(
      publish_diagnostics.text_document,
      FileState::diagnostics_mut,
      publish_diagnostics.diagnostics,
    )
  }

  pub fn on_notification(&mut self, notification: Notification) -> Option<WidgetSet> {
    match notification {
      Notification::FileProgress(file_progress) => self.on_file_progress(file_progress).some(),
      Notification::PublishDiagnostics(publish_diagnostics) => self.on_publish_diagnostics(publish_diagnostics).some(),
      Notification::Other => None,
    }
  }

  pub fn on_get_plain_goals_response(&mut self, get_plain_goals_response: GetPlainGoalsResponse) -> WidgetSet {
    self.plain_goals.mem_replace(get_plain_goals_response.result).mem_drop();

    self.build()
  }

  pub fn on_hover_file_response(&mut self, hover_file_response: HoverFileResponse) -> WidgetSet {
    self
      .hover_file_result
      .mem_replace(hover_file_response.result)
      .mem_drop();

    self.build()
  }
}
