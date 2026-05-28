use std::{ops::Range, sync::LazyLock};

use arborium_lean as lean;
use getset::{Getters, MutGetters};
use mkutils::{Rope, ScrollView, ScrollViewState, ScrollWhen, Utils};
use ratatui::{
  Frame,
  layout::{Margin, Rect},
  style::{Style, Styled, Stylize},
  text::{Line, Span},
  widgets::Block,
};
use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_md::{
  HIGHLIGHT_QUERY_BLOCK, HIGHLIGHT_QUERY_INLINE, INJECTION_QUERY_BLOCK, INJECTION_QUERY_INLINE, INLINE_LANGUAGE,
  LANGUAGE,
};

use crate::{
  highlight_captures::{
    HIGHLIGHT_ATTRIBUTE, HIGHLIGHT_CHARACTER, HIGHLIGHT_COMMENT, HIGHLIGHT_CONSTANT, HIGHLIGHT_CONSTRUCTOR,
    HIGHLIGHT_FUNCTION, HIGHLIGHT_KEYWORD, HIGHLIGHT_MARKUP_RAW, HIGHLIGHT_NAMES, HIGHLIGHT_NUMBER, HIGHLIGHT_OPERATOR,
    HIGHLIGHT_PROPERTY, HIGHLIGHT_PUNCTUATION, HIGHLIGHT_STRING, HIGHLIGHT_STRING_ESCAPE, HIGHLIGHT_TEXT_EMPHASIS,
    HIGHLIGHT_TEXT_LITERAL, HIGHLIGHT_TEXT_REFERENCE, HIGHLIGHT_TEXT_STRONG, HIGHLIGHT_TEXT_TITLE, HIGHLIGHT_TEXT_URI,
    HIGHLIGHT_TYPE, HIGHLIGHT_WARNING,
  },
  types::{Position, Severity, Utf16},
  widget_set_builder::WidgetSetBuilder,
};

// Tree-sitter highlight queries are compiled once, then reused by each hover
// render.
static MARKDOWN_HIGHLIGHT_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
  let mut config = HighlightConfiguration::new(
    LANGUAGE.into(),
    "markdown",
    HIGHLIGHT_QUERY_BLOCK,
    INJECTION_QUERY_BLOCK,
    "",
  )
  .expect("markdown highlight query should be valid");

  config.configure(&HIGHLIGHT_NAMES);

  config
});

// Inline markdown is a separate Tree-sitter grammar injected by the block
// grammar.
static MARKDOWN_INLINE_HIGHLIGHT_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
  let mut config = HighlightConfiguration::new(
    INLINE_LANGUAGE.into(),
    "markdown_inline",
    HIGHLIGHT_QUERY_INLINE,
    INJECTION_QUERY_INLINE,
    "",
  )
  .expect("markdown inline highlight query should be valid");

  config.configure(&HIGHLIGHT_NAMES);

  config
});

// Lean code fences inside markdown are injected into this grammar when the info
// string is `lean` or `lean4`.
static LEAN_HIGHLIGHT_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
  let mut config = HighlightConfiguration::new(
    lean::language().into(),
    "lean",
    lean::HIGHLIGHTS_QUERY,
    lean::INJECTIONS_QUERY,
    lean::LOCALS_QUERY,
  )
  .expect("lean highlight query should be valid");

  config.configure(&HIGHLIGHT_NAMES);

  config
});

struct MarkdownSource<'a> {
  source: &'a str,
  rope: Rope,
  line_index: usize,
  line_end_byte: usize,
}

impl<'a> MarkdownSource<'a> {
  fn new(source: &'a str) -> Self {
    let rope = Rope::from(source);
    let line_index = 0;
    let line_end_byte = Self::line_end_byte(&rope, line_index, source.len());

    Self {
      source,
      rope,
      line_index,
      line_end_byte,
    }
  }

  fn line_end_byte(rope: &Rope, line_index: usize, source_len: usize) -> usize {
    rope.line_info(line_index).map_or(source_len, |(offset, summary)| {
      offset.length.bytes.saturating_add(summary.length.bytes)
    })
  }

  fn advance_line(&mut self) {
    self.line_index = self.line_index.incremented();
    self.line_end_byte = Self::line_end_byte(&self.rope, self.line_index, self.source.len());
  }

  fn push_range(
    &mut self,
    lines: &mut Vec<Line<'static>>,
    spans: &mut Vec<Span<'static>>,
    range: Range<usize>,
    style: Style,
  ) {
    let mut start = range.start;

    while start < range.end {
      let end = range.end.min(self.line_end_byte);
      let segment = &self.source[start..end];

      if let Some(line_segment) = segment.strip_suffix('\n') {
        spans.push(Span::styled(line_segment.to_owned(), style));
        lines.push(std::mem::take(spans).into());
        self.advance_line();
      } else {
        spans.push(Span::styled(segment.to_owned(), style));
      }

      start = end;
    }
  }
}

#[derive(Getters)]
#[get = "pub"]
pub struct View {
  lines: Vec<Line<'static>>,
  block: Block<'static>,
  scroll_view_state: ScrollViewState,
}

impl View {
  const CONTENT_AREA_MARGIN: Margin = Margin::new(1, 1);
  const RENDER_SCROLL_BARS_TIMEOUT: Option<std::time::Duration> = Some(std::time::Duration::from_secs(1));
  const SCROLL_VIEW_STATE: ScrollViewState =
    ScrollViewState::new(ScrollWhen::ForLargeContent, Self::RENDER_SCROLL_BARS_TIMEOUT);
  const STYLE_BLOCK_BORDER: Style = Style::new().white().bold();
  const STYLE_BLOCK_TITLE: Style = Style::new().dark_gray();

  pub fn new(title: &str, lines: Vec<Line<'static>>) -> Self {
    let block = Self::create_block(title);
    let mut scroll_view_state = Self::SCROLL_VIEW_STATE;

    scroll_view_state.set_latest_content_size(lines.content_size());

    Self {
      lines,
      block,
      scroll_view_state,
    }
  }

  fn create_block(title: &str) -> Block<'static> {
    let title = std::format!(" {title} ")
      .convert::<Line>()
      .centered()
      .set_style(Self::STYLE_BLOCK_TITLE);

    Block::bordered().border_style(Self::STYLE_BLOCK_BORDER).title(title)
  }
}

impl ScrollView for View {
  fn scroll_view_state(&self) -> &ScrollViewState {
    &self.scroll_view_state
  }

  fn scroll_view_state_mut(&mut self) -> &mut ScrollViewState {
    &mut self.scroll_view_state
  }

  fn content_area(&self, scroll_view_area: Rect) -> Rect {
    scroll_view_area.inner(Self::CONTENT_AREA_MARGIN)
  }

  fn render_content(&self, frame: &mut Frame, content_area: Rect) {
    let scroll_offset = self.scroll_view_state.scroll_offset();
    let rows = scroll_offset.y.range_from_len(content_area.height.into());
    let cols = scroll_offset.x.range_from_len(content_area.width.into());
    let line_and_row_area_pairs = self.lines[rows.clamp()].iter().zip(content_area.rows());

    for (line, row_area) in line_and_row_area_pairs {
      line.subline(cols.clone()).render_to(frame, row_area);
    }
  }

  fn render_misc(&self, frame: &mut Frame, scroll_view_area: Rect) {
    self.block.ref_immut().render_to(frame, scroll_view_area);
  }

  fn scroll_bar_style(&self) -> Style {
    Self::STYLE_BLOCK_BORDER
  }
}

#[derive(MutGetters)]
#[get_mut = "pub"]
pub struct WidgetSet {
  goals: View,
  hover_info: View,
  messages: View,
}

impl WidgetSet {
  const MESSAGE_NO_GOALS: &str = "No goals";
  const MESSAGE_NO_HOVER: &str = "No information available";
  const MESSAGE_NO_MESSAGE: &str = "No messages";
  const TITLE_GOALS: &str = "Goals";
  const TITLE_HOVER_INFO: &str = "Hover Info";
  const TITLE_MESSAGES: &str = "Messages";
  const TURNSTILE: &str = "⊢ ";

  pub fn new(widget_set_builder: &WidgetSetBuilder) -> Self {
    let goals = Self::create_goals_view(widget_set_builder);
    let hover_info = Self::create_hover_info_view(widget_set_builder);
    let messages = Self::create_messages_view(widget_set_builder);

    Self {
      goals,
      hover_info,
      messages,
    }
  }

  fn hypothesis_line(line: &str) -> Line<'static> {
    let Some((prefix, colon, suffix)) = line.find_substr(" : ") else {
      return line.to_owned().white().into();
    };
    let prefix = prefix.to_owned();
    let colon = colon.to_owned();
    let suffix = suffix.to_owned();

    prefix.green().add_span(colon.dark_gray()).add_span(suffix.white())
  }

  fn goal_line(line: &str) -> Line<'static> {
    if let Some(suffix) = line.strip_prefix(Self::TURNSTILE) {
      Self::TURNSTILE.blue().bold().add_span(suffix.to_owned().cyan())
    } else {
      Self::hypothesis_line(line)
    }
  }

  fn goals_view_lines(widget_set_builder: &WidgetSetBuilder) -> Vec<Line<'static>> {
    let Some(plain_goals) = &widget_set_builder.plain_goals() else {
      return Self::MESSAGE_NO_GOALS.dim().convert::<Line>().singleton();
    };

    if plain_goals.goals.is_empty() {
      return Self::MESSAGE_NO_GOALS.dim().convert::<Line>().singleton();
    }

    let mut lines = Vec::new();
    let multiple_goals = 1 < plain_goals.goals.len();

    for (index, goal_str) in plain_goals.goals.iter().enumerate() {
      if index.is_positive() {
        lines.push(Line::default());
      }

      if multiple_goals {
        let case_num = index.incremented();

        std::format!("case {case_num}")
          .yellow()
          .bold()
          .convert::<Line>()
          .push_to(&mut lines);
      }

      goal_str.lines().map(Self::goal_line).push_all_to(&mut lines);
    }

    lines
  }

  fn create_goals_view(widget_set_builder: &WidgetSetBuilder) -> View {
    let lines = Self::goals_view_lines(widget_set_builder);

    View::new(Self::TITLE_GOALS, lines)
  }

  const fn tree_sitter_highlight_style(highlight: Highlight) -> Style {
    match highlight.0 {
      HIGHLIGHT_ATTRIBUTE | HIGHLIGHT_PROPERTY => Style::new().blue(),
      HIGHLIGHT_CHARACTER | HIGHLIGHT_STRING | HIGHLIGHT_TEXT_LITERAL | HIGHLIGHT_MARKUP_RAW => Style::new().yellow(),
      HIGHLIGHT_COMMENT => Style::new().dark_gray().italic(),
      HIGHLIGHT_CONSTANT | HIGHLIGHT_NUMBER => Style::new().cyan(),
      HIGHLIGHT_CONSTRUCTOR | HIGHLIGHT_FUNCTION | HIGHLIGHT_TEXT_REFERENCE => Style::new().green(),
      HIGHLIGHT_KEYWORD | HIGHLIGHT_STRING_ESCAPE => Style::new().magenta(),
      HIGHLIGHT_OPERATOR | HIGHLIGHT_PUNCTUATION => Style::new().dark_gray(),
      HIGHLIGHT_TEXT_EMPHASIS => Style::new().white().italic(),
      HIGHLIGHT_TEXT_STRONG => Style::new().white().bold(),
      HIGHLIGHT_TEXT_TITLE | HIGHLIGHT_TYPE => Style::new().cyan().bold(),
      HIGHLIGHT_TEXT_URI => Style::new().blue().underlined(),
      HIGHLIGHT_WARNING => Style::new().yellow().bold(),
      _ => Style::new().white(),
    }
  }

  fn markdown_lines(value: &str) -> Vec<Line<'static>> {
    let mut highlighter = Highlighter::new();
    let Ok(events) = highlighter.highlight(
      &MARKDOWN_HIGHLIGHT_CONFIG,
      value.as_bytes(),
      None,
      |language| match language {
        "markdown_inline" => Some(&*MARKDOWN_INLINE_HIGHLIGHT_CONFIG),
        "lean" | "lean4" => Some(&*LEAN_HIGHLIGHT_CONFIG),
        _ => None,
      },
    ) else {
      return value.lines().map(|line| line.to_owned().white().into()).collect();
    };
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut source = MarkdownSource::new(value);
    let mut style_stack = vec![Style::new().white()];

    // Highlight events are byte ranges plus start/end markers. Track the active
    // style stack and use rope byte offsets to split ranges back into ratatui
    // lines at the same byte boundaries Tree-sitter reports.
    for event in events.flatten() {
      match event {
        HighlightEvent::Source { start, end } => {
          let style = style_stack.last().copied().unwrap_or_else(Style::new);

          source.push_range(&mut lines, &mut spans, start..end, style);
        }
        HighlightEvent::HighlightStart(highlight) => style_stack.push(Self::tree_sitter_highlight_style(highlight)),
        HighlightEvent::HighlightEnd => {
          style_stack.pop();
        }
      }
    }

    if !spans.is_empty() {
      lines.push(spans.into());
    }

    lines
  }

  fn hover_info_lines(widget_set_builder: &WidgetSetBuilder) -> Vec<Line<'static>> {
    if let Some(hover_file_result) = &widget_set_builder.hover_file_result() {
      Self::markdown_lines(&hover_file_result.contents.value)
    } else {
      Self::MESSAGE_NO_HOVER.dim().convert::<Line>().singleton()
    }
  }

  fn create_hover_info_view(widget_set_builder: &WidgetSetBuilder) -> View {
    let lines = Self::hover_info_lines(widget_set_builder);

    View::new(Self::TITLE_HOVER_INFO, lines)
  }

  fn push_message_lines(
    lines: &mut Vec<Line<'static>>,
    file_name: &str,
    position: Position<Utf16>,
    severity: Severity,
    message: &str,
  ) {
    let file_name = file_name.to_owned().white().bold();
    let row = position.line.incremented();
    let col = position.character.incremented();
    let suffix = std::format!(":{row}:{col}").dark_gray();
    let color = severity.color();
    let label = severity.label().bold().fg(color);
    let line_1 = file_name.add_span(suffix).add_span(" ").add_span(label);
    let line_2 = "  ".add_span(message.to_owned()).fg(color);
    let line_3 = Line::default();

    lines.push(line_1);
    lines.push(line_2);
    lines.push(line_3);
  }

  fn message_lines(widget_set_builder: &WidgetSetBuilder) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut rendered_message = false;

    for (text_document, file_state) in widget_set_builder.file_states() {
      let file_name = text_document.file_name();

      for processing in file_state.processing() {
        Self::push_message_lines(
          &mut lines,
          file_name,
          processing.range.start,
          processing.kind.severity(),
          processing.kind.message(),
        );
        rendered_message.set_true();
      }

      for diagnostic in file_state.diagnostics() {
        Self::push_message_lines(
          &mut lines,
          file_name,
          diagnostic.range.start,
          diagnostic.severity,
          &diagnostic.message,
        );
        rendered_message.set_true();
      }
    }

    if !rendered_message {
      Self::MESSAGE_NO_MESSAGE.dim().convert::<Line>().push_to(&mut lines);
    }

    lines
  }

  fn create_messages_view(widget_set_builder: &WidgetSetBuilder) -> View {
    let lines = Self::message_lines(widget_set_builder);

    View::new(Self::TITLE_MESSAGES, lines)
  }
}
