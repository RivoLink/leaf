use crate::theme::MarkdownTheme;
use ratatui::text::Line;

use super::width::display_width;
use super::with_link_marker;

#[derive(Clone)]
pub(crate) struct LinkSpan {
    pub line_idx: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub url: String,
}

pub(super) fn build_link_spans(
    lines: &[Line<'_>],
    link_urls: &[String],
    theme: &MarkdownTheme,
) -> Vec<LinkSpan> {
    with_link_marker(|marker| build_link_spans_with_marker(lines, link_urls, theme, marker))
}

fn build_link_spans_with_marker(
    lines: &[Line<'_>],
    link_urls: &[String],
    theme: &MarkdownTheme,
    marker: &str,
) -> Vec<LinkSpan> {
    let marker_width = display_width(marker);
    let mut spans = Vec::new();
    let mut url_idx = 0;
    for (line_idx, line) in lines.iter().enumerate() {
        if url_idx >= link_urls.len() {
            break;
        }
        let mut col = 0usize;
        let mut in_link = false;
        let mut link_start = 0usize;
        for span in &line.spans {
            let w = display_width(span.content.as_ref());
            if span.content.as_ref() == marker && span.style.fg == Some(theme.link_icon) {
                if in_link && url_idx < link_urls.len() {
                    spans.push(LinkSpan {
                        line_idx,
                        start_col: link_start,
                        end_col: col,
                        url: link_urls[url_idx].clone(),
                    });
                    url_idx += 1;
                }
                in_link = true;
                link_start = col;
            } else if in_link {
                let opens_label = col == link_start + marker_width && span.style.bg.is_some();
                let is_link_text = span.style.fg == Some(theme.link_text) || opens_label;
                if !is_link_text {
                    if url_idx < link_urls.len() {
                        spans.push(LinkSpan {
                            line_idx,
                            start_col: link_start,
                            end_col: col,
                            url: link_urls[url_idx].clone(),
                        });
                        url_idx += 1;
                    }
                    in_link = false;
                }
            }
            col += w;
        }
        if in_link && url_idx < link_urls.len() {
            spans.push(LinkSpan {
                line_idx,
                start_col: link_start,
                end_col: col,
                url: link_urls[url_idx].clone(),
            });
            url_idx += 1;
        }
    }
    spans
}
