use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;

use super::width::{rendered_cluster_width, rendered_span_width};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct LinkId(pub(crate) usize);

pub(super) fn register_link(link_urls: &mut Vec<String>, url: &str) -> LinkId {
    link_urls.push(url.to_owned());
    LinkId(link_urls.len() - 1)
}

#[derive(Clone)]
pub(super) struct LinkedSpan {
    pub(super) span: Span<'static>,
    pub(super) link_id: Option<LinkId>,
}

impl LinkedSpan {
    pub(super) fn new(span: Span<'static>, link_id: Option<LinkId>) -> Self {
        Self { span, link_id }
    }
}

pub(crate) struct LinkSpan {
    pub line_idx: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub link_id: LinkId,
}

pub(super) fn emit_linked_line(
    lines: &mut Vec<Line<'static>>,
    ranges: &mut Vec<LinkSpan>,
    linked_spans: Vec<LinkedSpan>,
) {
    let line_idx = lines.len();
    let mut col = 0usize;
    let mut spans = Vec::with_capacity(linked_spans.len());

    let owned_end = linked_spans.iter().rposition(|span| span.link_id.is_some());
    for (index, linked) in linked_spans.into_iter().enumerate() {
        let width = if owned_end.is_some_and(|end| index <= end) {
            rendered_span_width(&linked.span)
        } else {
            0
        };
        if let Some(link_id) = linked.link_id.filter(|_| width > 0) {
            let end_col = col + width;
            match ranges.last_mut() {
                Some(previous)
                    if previous.line_idx == line_idx
                        && previous.link_id == link_id
                        && previous.end_col == col =>
                {
                    previous.end_col = end_col;
                }
                _ => ranges.push(LinkSpan {
                    link_id,
                    line_idx,
                    start_col: col,
                    end_col,
                }),
            }
        }
        col += width;
        spans.push(linked.span);
    }

    lines.push(Line::from(spans));
}

// Footnotes are rendered first and rewrapped later; recover owners from their ranges.
pub(super) fn restore_linked_spans(
    line_idx: usize,
    line: &Line<'_>,
    ranges: &[LinkSpan],
) -> Vec<LinkedSpan> {
    let start = ranges.partition_point(|range| range.line_idx < line_idx);
    let end = ranges.partition_point(|range| range.line_idx <= line_idx);
    let ranges = &ranges[start..end];
    let mut range_index = 0;
    let mut restored = Vec::new();
    let mut col = 0usize;

    for span in &line.spans {
        let style = span.style;
        if span.content.is_empty() {
            restored.push(LinkedSpan::new(Span::styled("", style), None));
            continue;
        }
        let mut current_owner = None;
        let mut current_text = String::new();

        for cluster in span.content.graphemes(true) {
            let width = rendered_cluster_width(cluster);
            let owner = if width == 0 {
                current_owner
            } else {
                while range_index < ranges.len() && ranges[range_index].end_col <= col {
                    range_index += 1;
                }
                ranges
                    .get(range_index)
                    .filter(|range| range.start_col <= col && col + width <= range.end_col)
                    .map(|range| range.link_id)
            };

            if current_owner == owner {
                current_text.push_str(cluster);
            } else {
                if !current_text.is_empty() {
                    restored.push(LinkedSpan::new(
                        Span::styled(std::mem::take(&mut current_text), style),
                        current_owner,
                    ));
                }
                current_owner = owner;
                current_text.push_str(cluster);
            }
            col += width;
        }

        if !current_text.is_empty() {
            restored.push(LinkedSpan::new(
                Span::styled(current_text, style),
                current_owner,
            ));
        }
    }

    restored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restoring_owners_advances_across_ranges_without_crossing_rows() {
        let line = Line::from(vec![Span::raw("p\tA"), Span::raw("é BC")]);
        let ranges = vec![
            LinkSpan {
                link_id: LinkId(8),
                line_idx: 1,
                start_col: 0,
                end_col: 9,
            },
            LinkSpan {
                link_id: LinkId(0),
                line_idx: 2,
                start_col: 1,
                end_col: 3,
            },
            LinkSpan {
                link_id: LinkId(1),
                line_idx: 2,
                start_col: 4,
                end_col: 6,
            },
            LinkSpan {
                link_id: LinkId(9),
                line_idx: 3,
                start_col: 0,
                end_col: 9,
            },
        ];
        let restored = restore_linked_spans(2, &line, &ranges);
        let actual: Vec<_> = restored
            .iter()
            .map(|span| (span.span.content.as_ref(), span.link_id))
            .collect();
        assert_eq!(
            actual,
            vec![
                ("p\t", None),
                ("A", Some(LinkId(0))),
                ("é", Some(LinkId(0))),
                (" ", None),
                ("BC", Some(LinkId(1)))
            ]
        );
        let unowned = restore_linked_spans(4, &line, &ranges);
        assert!(unowned.iter().all(|span| span.link_id.is_none()));
        assert_eq!(
            unowned
                .iter()
                .map(|span| span.span.content.as_ref())
                .collect::<String>(),
            "p\tAé BC"
        );
    }
}
