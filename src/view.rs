use std::time::Duration;

use getset::Getters;
use mkutils::{ScrollView, ScrollViewState, ScrollWhen, Utils};
use ratatui::{
  Frame,
  layout::{Margin, Rect},
  style::{Style, Styled},
  text::Line,
  widgets::{Block, BorderType},
};

#[derive(Getters)]
#[get = "pub"]
pub struct View {
  lines: Vec<Line<'static>>,
  block: Block<'static>,
  scroll_view_state: ScrollViewState,
}

impl View {
  const BORDER_TYPE: BorderType = BorderType::Rounded;
  const CONTENT_AREA_MARGIN: Margin = Margin::new(1, 1);
  const RENDER_SCROLL_BARS_TIMEOUT: Option<Duration> = Some(Duration::from_secs(1));
  const SCROLL_VIEW_STATE: ScrollViewState =
    ScrollViewState::new(ScrollWhen::ForLargeContent, Self::RENDER_SCROLL_BARS_TIMEOUT);
  const STYLE_BLOCK_BORDER: Style = Style::new().white().bold();
  const STYLE_BLOCK_TITLE: Style = Style::new().dark_gray();

  pub fn new(title: &str, lines: Vec<Line<'static>>) -> Self {
    let block = Self::create_block(title);
    let mut scroll_view_state = Self::SCROLL_VIEW_STATE;

    // NOTE: see [latest-content-size-abb4a0]
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

    Block::bordered()
      .border_style(Self::STYLE_BLOCK_BORDER)
      .border_type(Self::BORDER_TYPE)
      .title(title)
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
    let line_and_row_area_pairs = self.lines.clamped_index(rows).iter().zip(content_area.rows());

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
