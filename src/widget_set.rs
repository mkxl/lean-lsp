use getset::MutGetters;
use mkutils::{SyntaxHighlighter, Utils};
use ratatui::{
  style::{Style, Stylize},
  text::Line,
};
use tree_sitter_highlight::Error as TreeSitterHighlightError;

use crate::{
  highlight_state::HighlightState,
  types::{Position, Severity, Utf16},
  view::View,
  widget_set_builder::WidgetSetBuilder,
};

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

  pub fn new(widget_set_builder: &mut WidgetSetBuilder) -> Self {
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
          .push_to(lines.ref_mut());
      }

      goal_str.lines().map(Self::goal_line).push_all_to(&mut lines);
    }

    lines
  }

  fn create_goals_view(widget_set_builder: &WidgetSetBuilder) -> View {
    let lines = Self::goals_view_lines(widget_set_builder);

    View::new(Self::TITLE_GOALS, lines)
  }

  fn markdown_lines(
    syntax_highlighter: &mut SyntaxHighlighter<Style>,
    value: &str,
  ) -> Result<Vec<Line<'static>>, TreeSitterHighlightError> {
    let mut highlight_state = HighlightState::new(value);

    syntax_highlighter.highlight(
      WidgetSetBuilder::LANGUAGE_NAME_MARKDOWN,
      value,
      highlight_state.ref_mut(),
    )
  }

  fn hover_info_lines(widget_set_builder: &mut WidgetSetBuilder) -> Vec<Line<'static>> {
    let Some((hover_file_result, syntax_highlighter)) = widget_set_builder.hover_file_result_and_syntax_highlighter()
    else {
      return Self::MESSAGE_NO_HOVER.dim().convert::<Line>().singleton();
    };

    match Self::markdown_lines(syntax_highlighter, &hover_file_result.contents.value) {
      Ok(markdown_lines) => markdown_lines,
      Err(_tree_sitter_err) => hover_file_result.contents.value.lines().map(Self::goal_line).collect(),
    }
  }

  fn create_hover_info_view(widget_set_builder: &mut WidgetSetBuilder) -> View {
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
