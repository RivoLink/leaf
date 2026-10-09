use super::App;
use crate::markdown::{display_width, LinkId, LinkSpan};
use std::collections::HashMap;

pub(super) fn link_spans_to_map(link_spans: Vec<LinkSpan>) -> HashMap<usize, Vec<LinkSpan>> {
    let mut map: HashMap<usize, Vec<LinkSpan>> = HashMap::new();
    for span in link_spans {
        map.entry(span.line_idx).or_default().push(span);
    }
    map
}

impl App {
    pub(crate) fn set_links(&mut self, link_urls: Vec<String>, link_spans: Vec<LinkSpan>) {
        self.link_urls = link_urls;
        self.link_spans_by_line = link_spans_to_map(link_spans);
    }

    pub(crate) fn link_url(&self, id: LinkId) -> Option<&str> {
        self.link_urls.get(id.0).map(String::as_str)
    }

    pub(crate) fn link_at_position(
        &self,
        col: u16,
        row: u16,
        padding: u16,
        sb_width: u16,
        gutter_width: u16,
    ) -> Option<&LinkSpan> {
        self.find_hovered_link(col, row, padding, sb_width, gutter_width)
            .and_then(|(line_idx, span_idx)| {
                self.link_spans_by_line
                    .get(&line_idx)
                    .and_then(|spans| spans.get(span_idx))
            })
    }

    pub(crate) fn find_hovered_link(
        &self,
        col: u16,
        row: u16,
        padding: u16,
        sb_width: u16,
        gutter_width: u16,
    ) -> Option<(usize, usize)> {
        let area = self.content_area;
        let inner_x = area.x + padding + gutter_width;
        let inner_w = area
            .width
            .saturating_sub(padding * 2)
            .saturating_sub(sb_width)
            .saturating_sub(gutter_width);

        if col < inner_x || col >= inner_x + inner_w || row < area.y || row >= area.y + area.height
        {
            return None;
        }

        let rel_col = (col - inner_x) as usize;
        let rel_row = (row - area.y) as usize;
        let content_width = inner_w.max(1) as usize;

        let mut visual_row = 0usize;
        for line_idx in self.scroll..self.total() {
            let line = &self.lines[line_idx];
            let line_width: usize = line
                .spans
                .iter()
                .map(|s| display_width(s.content.as_ref()))
                .sum();
            let wrapped_lines = if line_width == 0 {
                1
            } else {
                line_width.div_ceil(content_width)
            };

            if rel_row < visual_row + wrapped_lines {
                let row_in_wrap = rel_row - visual_row;
                let char_col = row_in_wrap * content_width + rel_col;
                if let Some(spans) = self.link_spans_by_line.get(&line_idx) {
                    if let Some(idx) = spans
                        .iter()
                        .position(|ls| char_col >= ls.start_col && char_col < ls.end_col)
                    {
                        return Some((line_idx, idx));
                    }
                }
                return None;
            }
            visual_row += wrapped_lines;
            if visual_row > area.height as usize {
                break;
            }
        }
        None
    }
}
