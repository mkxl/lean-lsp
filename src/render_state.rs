use std::collections::HashMap;

use camino::Utf8PathBuf;

use crate::{
  notification::{FileProgress, Notification, PublishDiagnostics},
  responses::{GetPlainGoalsResponse, HoverFileResponse},
  types::{Diagnostic, HoverFileResult, PlainGoals},
};

#[derive(Default)]
pub struct FileState {
  pub diagnostics: Vec<Diagnostic>,
  pub is_processing: bool,
}

pub struct RenderState {
  file_states: HashMap<Utf8PathBuf, FileState>,
  plain_goals: Option<PlainGoals>,
  hover: Option<HoverFileResult>,
}

impl RenderState {
  pub fn new() -> Self {
    Self {
      file_states: HashMap::new(),
      plain_goals: None,
      hover: None,
    }
  }

  pub const fn plain_goals(&self) -> Option<&PlainGoals> {
    self.plain_goals.as_ref()
  }

  pub const fn hover(&self) -> Option<&HoverFileResult> {
    self.hover.as_ref()
  }

  pub const fn file_states(&self) -> &HashMap<Utf8PathBuf, FileState> {
    &self.file_states
  }

  fn on_file_progress(&mut self, file_progress: &FileProgress) {
    tracing::info!(?file_progress);

    let uri = file_progress.text_document.uri();
    let Some(filepath) = uri.strip_prefix("file://") else { return };
    let is_processing = !file_progress.processing.is_empty();

    self
      .file_states
      .entry(Utf8PathBuf::from(filepath))
      .or_default()
      .is_processing = is_processing;
  }

  fn on_publish_diagnostics(&mut self, publish_diagnostics: &PublishDiagnostics) {
    tracing::info!(?publish_diagnostics);

    let uri = publish_diagnostics.text_document.uri();
    let Some(filepath) = uri.strip_prefix("file://") else { return };

    self
      .file_states
      .entry(Utf8PathBuf::from(filepath))
      .or_default()
      .diagnostics
      .clone_from(&publish_diagnostics.diagnostics);
  }

  pub fn on_notification(&mut self, notification: &Notification) {
    match notification {
      Notification::FileProgress(file_progress) => self.on_file_progress(file_progress),
      Notification::PublishDiagnostics(publish_diagnostics) => self.on_publish_diagnostics(publish_diagnostics),
      Notification::Other => {}
    }
  }

  pub fn on_get_plain_goals_response(&mut self, get_plain_goals_response: &GetPlainGoalsResponse) {
    tracing::info!(?get_plain_goals_response);

    self.plain_goals.clone_from(&get_plain_goals_response.result);
  }

  pub fn on_hover_file_response(&mut self, hover_file_response: &HoverFileResponse) {
    tracing::info!(?hover_file_response);

    self.hover = Some(hover_file_response.result.clone());
  }
}
