use std::{borrow::Cow, collections::HashMap, ops::Range};

use arborium_lean as lean;
use mkutils::{ColorScheme, Highlight as MkHighlight, Utils};
use ratatui::{
  style::Style,
  text::{Line, Span},
};
use tree_sitter_highlight::HighlightConfiguration;
use tree_sitter_md::{
  HIGHLIGHT_QUERY_BLOCK, HIGHLIGHT_QUERY_INLINE, INJECTION_QUERY_BLOCK, INJECTION_QUERY_INLINE, INLINE_LANGUAGE,
  LANGUAGE,
};

pub struct RatatuiHighlight<'a> {
  source: &'a str,
  lines: Vec<Line<'static>>,
  spans: Vec<Span<'static>>,
}

impl<'a> RatatuiHighlight<'a> {
  pub const fn new(source: &'a str) -> Self {
    let lines = Vec::new();
    let spans = Vec::new();

    Self { source, lines, spans }
  }

  pub fn color_scheme() -> ColorScheme<Style> {
    ColorScheme::new(Style::new().white()).insert_all(Self::style_from_capture_names())
  }

  fn style_from_capture_names() -> HashMap<Cow<'static, str>, Style> {
    mkutils::map! {
      "attribute": Style::new().blue(),
      "character": Style::new().yellow(),
      "comment": Style::new().dark_gray().italic(),
      "constant": Style::new().cyan(),
      "constructor": Style::new().green(),
      "function": Style::new().green(),
      "keyword": Style::new().magenta(),
      "markup.raw": Style::new().yellow(),
      "number": Style::new().cyan(),
      "operator": Style::new().dark_gray(),
      "property": Style::new().blue(),
      "punctuation": Style::new().dark_gray(),
      "string": Style::new().yellow(),
      "string.escape": Style::new().magenta(),
      "text.emphasis": Style::new().white().italic(),
      "text.literal": Style::new().yellow(),
      "text.reference": Style::new().green(),
      "text.strong": Style::new().white().bold(),
      "text.title": Style::new().cyan().bold(),
      "text.uri": Style::new().blue().underlined(),
      "type": Style::new().cyan().bold(),
      "warning": Style::new().yellow().bold(),
    }
  }

  pub fn markdown_highlight_configuration() -> HighlightConfiguration {
    HighlightConfiguration::new(
      LANGUAGE.into(),
      "markdown",
      HIGHLIGHT_QUERY_BLOCK,
      INJECTION_QUERY_BLOCK,
      "",
    )
    .expect("markdown highlight query should be valid")
  }

  pub fn markdown_inline_highlight_configuration() -> HighlightConfiguration {
    HighlightConfiguration::new(
      INLINE_LANGUAGE.into(),
      "markdown_inline",
      HIGHLIGHT_QUERY_INLINE,
      INJECTION_QUERY_INLINE,
      "",
    )
    .expect("markdown inline highlight query should be valid")
  }

  pub fn lean_highlight_configuration() -> HighlightConfiguration {
    HighlightConfiguration::new(
      lean::language().into(),
      "lean",
      lean::HIGHLIGHTS_QUERY,
      lean::INJECTIONS_QUERY,
      lean::LOCALS_QUERY,
    )
    .expect("lean highlight query should be valid")
  }

  fn push_range(&mut self, range: Range<usize>, style: Style) {
    let mut start = range.start;

    while start < range.end {
      let segment = &self.source[start..range.end];

      if let Some(newline_index) = segment.find('\n') {
        let end = start.saturating_add(newline_index);

        Span::styled(self.source[start..end].to_owned(), style).push_to(&mut self.spans);
        std::mem::take(&mut self.spans)
          .convert::<Line>()
          .push_to(&mut self.lines);
        start = end.incremented();
      } else {
        Span::styled(segment.to_owned(), style).push_to(&mut self.spans);
        break;
      }
    }
  }
}

impl MkHighlight<Style> for RatatuiHighlight<'_> {
  type Output = Vec<Line<'static>>;

  fn highlight(&mut self, begin_byte_index: usize, end_byte_index: usize, style: &Style) {
    self.push_range(begin_byte_index..end_byte_index, *style);
  }

  fn finish(&mut self) -> Self::Output {
    if !self.spans.is_empty() {
      std::mem::take(&mut self.spans)
        .convert::<Line>()
        .push_to(&mut self.lines);
    }

    std::mem::take(&mut self.lines)
  }
}
