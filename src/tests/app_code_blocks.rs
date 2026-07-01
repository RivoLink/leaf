use super::{test_assets, test_md_theme};
use crate::app::App;
use crate::markdown::parse_markdown;
use ratatui::layout::Rect;

fn build_app_with_mixed_blocks() -> App {
    let src = "```rust\nfn main() {}\n```\n\n\
               ```mermaid\ngraph TD;\nA-->B;\n```\n\n\
               ```latex\nE = mc^2\n```\n\n\
               $$ F = ma $$\n";
    let (ss, theme) = test_assets();
    let parsed = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true);
    let mut app = App::new(
        parsed.lines,
        parsed.toc,
        "test".to_string(),
        false,
        false,
        None,
        None,
    );
    app.set_code_blocks(parsed.code_blocks);
    app.content_area = Rect::new(0, 0, 80, 200);
    app
}

#[test]
fn enter_code_select_mode_picks_first_visible_block() {
    let mut app = build_app_with_mixed_blocks();
    app.enter_code_select_mode();
    assert_eq!(app.code_select, Some(0));
}

#[test]
fn code_select_next_cycles_through_all_blocks_including_mermaid_latex_and_display_math() {
    let mut app = build_app_with_mixed_blocks();
    app.enter_code_select_mode();
    assert_eq!(app.code_select, Some(0));
    app.code_select_next();
    assert_eq!(app.code_select, Some(1));
    app.code_select_next();
    assert_eq!(app.code_select, Some(2));
    app.code_select_next();
    assert_eq!(app.code_select, Some(3));
    app.code_select_next();
    assert_eq!(app.code_select, Some(0));
}

#[test]
fn copy_selected_code_block_works_for_mermaid_latex_and_display_math() {
    for idx in [1, 2, 3] {
        let mut app = build_app_with_mixed_blocks();
        app.code_select = Some(idx);
        app.copy_selected_code_block();
        assert!(
            app.code_block_flash().is_some(),
            "copy at idx={idx} should set a flash"
        );
        assert_eq!(app.code_select, None);
    }
}

#[test]
fn code_block_at_resolves_mermaid_latex_and_display_math() {
    let app = build_app_with_mixed_blocks();
    let mermaid = &app.code_blocks[1];
    let latex = &app.code_blocks[2];
    let display_math = &app.code_blocks[3];
    let mermaid_mid = (mermaid.rendered_start + mermaid.rendered_end) / 2;
    let latex_mid = (latex.rendered_start + latex.rendered_end) / 2;
    let display_math_mid = (display_math.rendered_start + display_math.rendered_end) / 2;
    assert_eq!(app.code_block_at(mermaid_mid, 0), Some(1));
    assert_eq!(app.code_block_at(latex_mid, 0), Some(2));
    assert_eq!(app.code_block_at(display_math_mid, 0), Some(3));
}
