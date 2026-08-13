use std::collections::HashMap;

use getset::{Getters, MutGetters};
use mkutils::{CaptureName, ColorScheme, Constructor, SyntaxHighlighter, Utils};
use ratatui::style::Style;
use serde::{Deserialize, Serialize};
use tree_sitter::QueryError;
use tree_sitter_highlight::HighlightConfiguration;

use crate::{
  info_view::InfoView,
  notification::{FileProgress, Notification, PublishDiagnostics},
  responses::{GetPlainGoalsResponse, HoverFileResponse},
  types::{Diagnostic, HoverFileResult, PlainGoals, Processing, TextDocument},
};

#[derive(Clone, Debug, Default, Deserialize, Getters, MutGetters, Serialize)]
#[getset(get = "pub", get_mut = "pub")]
pub struct FileState {
  diagnostics: Vec<Diagnostic>,
  processing: Vec<Processing>,
}

#[derive(Clone, Debug, Default, Deserialize, Getters, MutGetters, Serialize)]
pub struct InfoViewData {
  #[get = "pub"]
  file_states: HashMap<TextDocument, FileState>,

  #[get = "pub"]
  plain_goals: Option<PlainGoals>,

  #[get = "pub"]
  hover_file_result: Option<HoverFileResult>,
}

#[derive(Constructor, Getters, MutGetters)]
#[constructor(from_values)]
pub struct InfoViewBuilder {
  #[get = "pub"]
  info_view_data: InfoViewData,

  #[get_mut = "pub"]
  syntax_highlighter: SyntaxHighlighter<Style>,
}

impl InfoViewBuilder {
  pub const LANGUAGE_NAME_MARKDOWN: &'static str = "markdown";

  const DEFAULT_STYLE: Style = Style::new().white();
  const LANGUAGE_NAME_MARKDOWN_INLINE: &'static str = "markdown_inline";
  const LANGUAGE_NAME_LEAN: &'static str = "lean";
  const LANGUAGE_NAME_LEAN_4: &'static str = "lean4";
  const LOCALS_QUERY_MARKDOWN: &'static str = "";
  const LOCALS_QUERY_MARKDOWN_INLINE: &'static str = "";

  pub fn new() -> Result<Self, QueryError> {
    Self::from_values(InfoViewData::default(), Self::new_syntax_highlighter()?).ok()
  }

  fn new_color_scheme() -> ColorScheme<Style> {
    ColorScheme::new(Self::DEFAULT_STYLE)
      .insert(CaptureName::ATTRIBUTE, Style::new().blue())
      .insert(CaptureName::CHARACTER, Style::new().yellow())
      .insert(CaptureName::COMMENT, Style::new().dark_gray().italic())
      .insert(CaptureName::CONSTANT, Style::new().cyan())
      .insert(CaptureName::CONSTRUCTOR, Style::new().green())
      .insert(CaptureName::FUNCTION, Style::new().green())
      .insert(CaptureName::KEYWORD, Style::new().magenta())
      .insert(CaptureName::MARKUP_RAW, Style::new().yellow())
      .insert(CaptureName::NUMBER, Style::new().cyan())
      .insert(CaptureName::OPERATOR, Style::new().dark_gray())
      .insert(CaptureName::PROPERTY, Style::new().blue())
      .insert(CaptureName::PUNCTUATION, Style::new().dark_gray())
      .insert(CaptureName::STRING, Style::new().yellow())
      .insert(CaptureName::STRING_ESCAPE, Style::new().magenta())
      .insert(CaptureName::TEXT_EMPHASIS, Style::new().white().italic())
      .insert(CaptureName::TEXT_LITERAL, Style::new().yellow())
      .insert(CaptureName::TEXT_REFERENCE, Style::new().green())
      .insert(CaptureName::TEXT_STRONG, Style::new().white().bold())
      .insert(CaptureName::TEXT_TITLE, Style::new().cyan().bold())
      .insert(CaptureName::TEXT_URI, Style::new().blue().underlined())
      .insert(CaptureName::TYPE, Style::new().cyan().bold())
      .insert(CaptureName::WARNING, Style::new().yellow().bold())
  }

  fn markdown_highlight_configuration() -> Result<HighlightConfiguration, QueryError> {
    HighlightConfiguration::new(
      tree_sitter_md::LANGUAGE.into(),
      Self::LANGUAGE_NAME_MARKDOWN,
      tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
      tree_sitter_md::INJECTION_QUERY_BLOCK,
      Self::LOCALS_QUERY_MARKDOWN,
    )
  }

  fn markdown_inline_highlight_configuration() -> Result<HighlightConfiguration, QueryError> {
    HighlightConfiguration::new(
      tree_sitter_md::INLINE_LANGUAGE.into(),
      Self::LANGUAGE_NAME_MARKDOWN_INLINE,
      tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
      tree_sitter_md::INJECTION_QUERY_INLINE,
      Self::LOCALS_QUERY_MARKDOWN_INLINE,
    )
  }

  fn lean_highlight_configuration() -> Result<HighlightConfiguration, QueryError> {
    HighlightConfiguration::new(
      arborium_lean::language().into(),
      Self::LANGUAGE_NAME_LEAN,
      arborium_lean::HIGHLIGHTS_QUERY,
      arborium_lean::INJECTIONS_QUERY,
      arborium_lean::LOCALS_QUERY,
    )
  }

  fn new_syntax_highlighter() -> Result<SyntaxHighlighter<Style>, QueryError> {
    SyntaxHighlighter::new(Self::new_color_scheme())
      .with_language(Self::markdown_highlight_configuration()?)
      .with_language(Self::markdown_inline_highlight_configuration()?)
      .with_language(Self::lean_highlight_configuration()?)
      .with_language_alias(Self::LANGUAGE_NAME_LEAN_4, Self::LANGUAGE_NAME_LEAN)
      .ok()
  }

  pub fn build(&mut self) -> InfoView {
    InfoView::new(self)
  }

  fn update_file_states<T>(
    &mut self,
    text_document: TextDocument,
    getter: impl FnOnce(&mut FileState) -> &mut T,
    new_value: T,
  ) -> InfoView {
    self
      .info_view_data
      .file_states
      .entry(text_document)
      .or_default()
      .pipe_into(getter)
      .mem_replace(new_value)
      .mem_drop();

    self.build()
  }

  fn on_file_progress(&mut self, file_progress: FileProgress) -> InfoView {
    self.update_file_states(
      file_progress.text_document,
      FileState::processing_mut,
      file_progress.processing,
    )
  }

  fn on_publish_diagnostics(&mut self, publish_diagnostics: PublishDiagnostics) -> InfoView {
    self.update_file_states(
      publish_diagnostics.text_document,
      FileState::diagnostics_mut,
      publish_diagnostics.diagnostics,
    )
  }

  pub fn on_notification(&mut self, notification: Notification) -> Option<InfoView> {
    match notification {
      Notification::FileProgress(file_progress) => self.on_file_progress(file_progress).some(),
      Notification::PublishDiagnostics(publish_diagnostics) => self.on_publish_diagnostics(publish_diagnostics).some(),
      Notification::Other => None,
    }
  }

  pub fn on_get_plain_goals_response(&mut self, get_plain_goals_response: GetPlainGoalsResponse) -> InfoView {
    self
      .info_view_data
      .plain_goals
      .mem_replace(get_plain_goals_response.result)
      .mem_drop();

    self.build()
  }

  pub fn on_hover_file_response(&mut self, hover_file_response: HoverFileResponse) -> InfoView {
    self
      .info_view_data
      .hover_file_result
      .mem_replace(hover_file_response.result)
      .mem_drop();

    self.build()
  }

  pub fn hover_file_result_and_syntax_highlighter(
    &mut self,
  ) -> Option<(&HoverFileResult, &mut SyntaxHighlighter<Style>)> {
    self
      .info_view_data
      .hover_file_result
      .as_ref()?
      .pair(self.syntax_highlighter.ref_mut())
      .some()
  }
}
