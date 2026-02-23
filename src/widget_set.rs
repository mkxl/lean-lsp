use getset::{Getters, MutGetters};
use mkutils::{ScrollView as MkutilsScrollView, ScrollViewState, ScrollWhen, Utils};
use ratatui::{
  style::{Style, Styled, Stylize},
  text::Line,
  widgets::Block,
};

use crate::{
  types::{Position, Severity, Utf16},
  widget_set_builder::WidgetSetBuilder,
};

pub type ScrollView = MkutilsScrollView<Vec<Line<'static>>>;

#[derive(MutGetters)]
#[get_mut = "pub"]
pub struct WidgetStateSet {
  goals: ScrollViewState,
  hover_info: ScrollViewState,
  messages: ScrollViewState,
}

impl WidgetStateSet {
  pub const fn new(scroll_when: ScrollWhen) -> Self {
    let goals = ScrollViewState::new(scroll_when);
    let hover_info = ScrollViewState::new(scroll_when);
    let messages = ScrollViewState::new(scroll_when);

    Self {
      goals,
      hover_info,
      messages,
    }
  }
}

#[derive(Getters)]
#[get = "pub"]
pub struct WidgetSet {
  goals: ScrollView,
  hover_info: ScrollView,
  messages: ScrollView,
}

impl WidgetSet {
  const MESSAGE_NO_GOALS: &str = "No goals";
  const MESSAGE_NO_HOVER: &str = "No information available";
  const MESSAGE_NO_MESSAGE: &str = "No messages";
  const STYLE_BLOCK_BORDER: Style = Style::new().white().bold();
  const STYLE_BLOCK_TITLE: Style = Style::new().dark_gray();
  const TITLE_GOALS: &str = "Goals";
  const TITLE_HOVER_INFO: &str = "Hover Info";
  const TITLE_MESSAGES: &str = "Messages";
  const TURNSTILE: &str = "⊢ ";

  pub fn new(widget_set_builder: &WidgetSetBuilder) -> Self {
    let goals = Self::new_goals(widget_set_builder);
    let hover_info = Self::new_hover_info(widget_set_builder);
    let messages = Self::new_messages(widget_set_builder);

    Self {
      goals,
      hover_info,
      messages,
    }
  }

  fn block(title: &str) -> Block<'static> {
    let title = std::format!(" {title} ")
      .convert::<Line>()
      .centered()
      .set_style(Self::STYLE_BLOCK_TITLE);

    Block::bordered().border_style(Self::STYLE_BLOCK_BORDER).title(title)
  }

  fn scroll_view(title: &str, lines: Vec<Line<'static>>) -> ScrollView {
    let block = Self::block(title);
    let mut scroll_view = ScrollView::new(lines);

    scroll_view.set_block(block);

    scroll_view
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

  fn goal_lines(widget_set_builder: &WidgetSetBuilder) -> Vec<Line<'static>> {
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

  fn new_goals(widget_set_builder: &WidgetSetBuilder) -> ScrollView {
    let lines = Self::goal_lines(widget_set_builder);

    Self::scroll_view(Self::TITLE_GOALS, lines)
  }

  fn hover_lines(widget_set_builder: &WidgetSetBuilder) -> Vec<Line<'static>> {
    if let Some(hover_file_result) = &widget_set_builder.hover_file_result() {
      hover_file_result.contents.value.lines().map(Self::goal_line).collect()
    } else {
      Self::MESSAGE_NO_HOVER.dim().convert::<Line>().singleton()
    }
  }

  fn new_hover_info(widget_set_builder: &WidgetSetBuilder) -> ScrollView {
    let lines = Self::hover_lines(widget_set_builder);

    Self::scroll_view(Self::TITLE_HOVER_INFO, lines)
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

  fn new_messages(widget_set_builder: &WidgetSetBuilder) -> ScrollView {
    let lines = Self::message_lines(widget_set_builder);

    Self::scroll_view(Self::TITLE_MESSAGES, lines)
  }
}
