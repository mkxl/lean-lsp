use derive_more::Constructor;

use crate::{
  notification::{FileProgress, Notification, PublishDiagnostics},
  responses::{GetPlainGoalsResponse, HoverFileResponse},
};

#[derive(Constructor)]
pub struct RenderState;

#[allow(clippy::needless_pass_by_ref_mut, clippy::unused_self)]
impl RenderState {
  fn on_file_progress(&mut self, file_progress: &FileProgress) {
    tracing::info!(?file_progress);
  }

  fn on_publish_diagnostics(&mut self, publish_diagnostics: &PublishDiagnostics) {
    tracing::info!(?publish_diagnostics);
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
  }

  pub fn on_hover_file_response(&mut self, hover_file_response: &HoverFileResponse) {
    tracing::info!(?hover_file_response);
  }
}
