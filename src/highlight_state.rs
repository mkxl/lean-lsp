use std::{iter::Peekable, vec::IntoIter as VecIntoIter};

use mkutils::{Highlight, Utils};
use ratatui::{
  style::{Style, Styled},
  text::{Line, Span},
};

pub struct HighlightState<'a> {
  source: &'a str,
  lines: Vec<Line<'static>>,
  spans: Vec<Span<'static>>,
  newline_byte_indices: Peekable<VecIntoIter<usize>>,
}

impl<'a> HighlightState<'a> {
  pub fn new(source: &'a str) -> Self {
    let lines = Vec::new();
    let spans = Vec::new();
    let newline_byte_indices = source
      .match_indices(Self::LF)
      .map(Utils::into_first)
      .collect::<Vec<usize>>()
      .into_iter()
      .peekable();

    Self {
      source,
      lines,
      spans,
      newline_byte_indices,
    }
  }

  fn get_highlight_end_byte_index_and_should_break(&mut self, end_byte_index: usize) -> (usize, bool) {
    let Some(&newline_byte_index) = self.newline_byte_indices.peek() else {
      return (end_byte_index, true);
    };

    if end_byte_index <= newline_byte_index {
      (end_byte_index, true)
    } else {
      (newline_byte_index, false)
    }
  }

  fn push_line(&mut self) {
    self.spans.mem_take().convert::<Line>().push_to(self.lines.ref_mut());
  }

  fn highlight_range(&mut self, mut begin_byte_index: usize, end_byte_index: usize, style: Style) {
    while begin_byte_index < end_byte_index {
      let (highlight_end_byte_index, should_break) = self.get_highlight_end_byte_index_and_should_break(end_byte_index);

      self.source[begin_byte_index..highlight_end_byte_index]
        .to_owned() // TODO-eed3e7
        .set_style(style)
        .push_to(self.spans.ref_mut());

      if should_break {
        break;
      }

      self.push_line();
      self.newline_byte_indices.next();

      begin_byte_index = highlight_end_byte_index.incremented();
    }
  }
}

impl Highlight<Style> for HighlightState<'_> {
  type Output = Vec<Line<'static>>;

  fn highlight(&mut self, begin_byte_index: usize, end_byte_index: usize, style: &Style) {
    self.highlight_range(begin_byte_index, end_byte_index, style.copied());
  }

  fn finish(&mut self) -> Self::Output {
    if !self.spans.is_empty() {
      self.push_line();
    }

    self.lines.mem_take()
  }
}
