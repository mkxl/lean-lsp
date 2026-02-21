use std::collections::HashMap;

use mkutils::Utils;

use crate::{
  notification::{FileProgress, Notification, PublishDiagnostics},
  responses::{GetPlainGoalsResponse, HoverFileResponse},
  types::{Diagnostic, HoverFileResult, PlainGoals, Processing, TextDocument},
};

#[derive(Default)]
pub struct FileState {
  pub diagnostics: Vec<Diagnostic>,
  pub processing: Vec<Processing>,
}

#[derive(Default)]
pub struct RenderState {
  pub file_states: HashMap<TextDocument, FileState>,
  pub plain_goals: Option<PlainGoals>,
  pub hover_file_result: Option<HoverFileResult>,
}

impl RenderState {
  fn file_state(&mut self, text_document: TextDocument) -> &mut FileState {
    self.file_states.entry(text_document).or_default()
  }

  fn on_file_progress(&mut self, file_progress: FileProgress) {
    self
      .file_state(file_progress.text_document)
      .processing
      .mem_replace(file_progress.processing)
      .mem_drop();
  }

  fn on_publish_diagnostics(&mut self, publish_diagnostics: PublishDiagnostics) {
    self
      .file_state(publish_diagnostics.text_document)
      .diagnostics
      .mem_replace(publish_diagnostics.diagnostics)
      .mem_drop();
  }

  pub fn on_notification(&mut self, notification: Notification) {
    match notification {
      Notification::FileProgress(file_progress) => self.on_file_progress(file_progress),
      Notification::PublishDiagnostics(publish_diagnostics) => self.on_publish_diagnostics(publish_diagnostics),
      Notification::Other => {}
    }
  }

  pub fn on_get_plain_goals_response(&mut self, get_plain_goals_response: GetPlainGoalsResponse) {
    self.plain_goals = get_plain_goals_response.result;
  }

  pub fn on_hover_file_response(&mut self, hover_file_response: HoverFileResponse) {
    self.hover_file_result = hover_file_response.result;
  }
}
