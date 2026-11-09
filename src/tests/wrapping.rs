use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{
    display_width, iter_cluster_widths, parse_markdown_with_width, truncate_display_width,
};

#[test]
fn iter_cluster_widths_matches_display_width_across_scripts() {
    for text in ["🗣️", "🍀️", "💡", "🎯", "🗣", "🍀", "abc", "天地不仁"] {
        let sum: usize = iter_cluster_widths(text).map(|(_, w)| w).sum();
        assert_eq!(
            sum,
            display_width(text),
            "cluster sum diverges from display_width for {text:?}"
        );
    }
}

#[test]
fn truncate_display_width_does_not_split_vs16_cluster() {
    let text = "🍀️ leaf note long";
    let out = truncate_display_width(text, 6);
    assert!(display_width(&out) <= 6);
    let base_present = out.contains('🍀');
    let vs16_present = out.contains('\u{FE0F}');
    assert_eq!(
        base_present, vs16_present,
        "VS16 must never appear without its base emoji: got {out:?}"
    );
}

#[test]
fn paragraph_with_vs16_emoji_fits_render_width() {
    let (ss, theme) = test_assets();
    let md = "\
🍀️ **Leaf note:** a paragraph with enough words to force wrapping across the render width
";
    let width = 40;
    let (lines, _, _, _) =
        parse_markdown_with_width(md, &ss, &theme, width, &test_md_theme(), false, true, true)
            .into();
    let rendered = rendered_non_empty_lines(&lines);
    assert!(!rendered.is_empty());
    for line in &rendered {
        assert!(
            display_width(line) <= width,
            "line wider than {width}: {:?} width={}",
            line,
            display_width(line)
        );
    }
}

#[test]
fn code_block_with_vs16_emoji_wraps_within_width() {
    let (ss, theme) = test_assets();
    let md = "```\naaaaaaaa 🍀️ bbbbbbbb cccccccc dddddddd eeeeeeee ffffffff\n```\n";
    let width = 30;
    let (lines, _, _, _) =
        parse_markdown_with_width(md, &ss, &theme, width, &test_md_theme(), false, true, true)
            .into();
    let rendered = rendered_non_empty_lines(&lines);
    assert!(!rendered.is_empty());
    for line in &rendered {
        assert!(
            display_width(line) <= width,
            "code line wider than {width}: {:?} width={}",
            line,
            display_width(line)
        );
    }
}
