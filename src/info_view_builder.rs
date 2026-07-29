use std::collections::HashMap;

use getset::{Getters, MutGetters};
use mkutils::{ColorScheme, Constructor, SyntaxHighlighter, Utils};
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

#[derive(Clone, Default, Deserialize, Getters, MutGetters, Serialize)]
#[getset(get = "pub", get_mut = "pub")]
pub struct FileState {
  diagnostics: Vec<Diagnostic>,
  processing: Vec<Processing>,
}

#[derive(Clone, Default, Deserialize, Getters, MutGetters, Serialize)]
pub struct InfoViewData {
  #[get = "pub"]
  file_states: HashMap<TextDocument, FileState>,

  #[get = "pub"]
  plain_goals: Option<PlainGoals>,

  #[get = "pub"]
  hover_file_result: Option<HoverFileResult>,
}

#[derive(Constructor, Getters, MutGetters, Serialize)]
#[constructor(from_values)]
pub struct InfoViewBuilder {
  #[get = "pub"]
  info_view_data: InfoViewData,

  #[get_mut = "pub"]
  #[serde(skip_serializing)]
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
      .insert(ColorScheme::<Style>::ATTRIBUTE, Style::new().blue())
      .insert(ColorScheme::<Style>::CHARACTER, Style::new().yellow())
      .insert(ColorScheme::<Style>::COMMENT, Style::new().dark_gray().italic())
      .insert(ColorScheme::<Style>::CONSTANT, Style::new().cyan())
      .insert(ColorScheme::<Style>::CONSTRUCTOR, Style::new().green())
      .insert(ColorScheme::<Style>::FUNCTION, Style::new().green())
      .insert(ColorScheme::<Style>::KEYWORD, Style::new().magenta())
      .insert(ColorScheme::<Style>::MARKUP_RAW, Style::new().yellow())
      .insert(ColorScheme::<Style>::NUMBER, Style::new().cyan())
      .insert(ColorScheme::<Style>::OPERATOR, Style::new().dark_gray())
      .insert(ColorScheme::<Style>::PROPERTY, Style::new().blue())
      .insert(ColorScheme::<Style>::PUNCTUATION, Style::new().dark_gray())
      .insert(ColorScheme::<Style>::STRING, Style::new().yellow())
      .insert(ColorScheme::<Style>::STRING_ESCAPE, Style::new().magenta())
      .insert(ColorScheme::<Style>::TEXT_EMPHASIS, Style::new().white().italic())
      .insert(ColorScheme::<Style>::TEXT_LITERAL, Style::new().yellow())
      .insert(ColorScheme::<Style>::TEXT_REFERENCE, Style::new().green())
      .insert(ColorScheme::<Style>::TEXT_STRONG, Style::new().white().bold())
      .insert(ColorScheme::<Style>::TEXT_TITLE, Style::new().cyan().bold())
      .insert(ColorScheme::<Style>::TEXT_URI, Style::new().blue().underlined())
      .insert(ColorScheme::<Style>::TYPE, Style::new().cyan().bold())
      .insert(ColorScheme::<Style>::WARNING, Style::new().yellow().bold())
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
    let mut syntax_highlighter = SyntaxHighlighter::new(Self::new_color_scheme());

    syntax_highlighter
      .add_language(Self::markdown_highlight_configuration()?)
      .add_language(Self::markdown_inline_highlight_configuration()?)
      .add_language(Self::lean_highlight_configuration()?)
      .add_language_alias(Self::LANGUAGE_NAME_LEAN_4, Self::LANGUAGE_NAME_LEAN);

    syntax_highlighter.ok()
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
