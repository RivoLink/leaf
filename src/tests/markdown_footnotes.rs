use super::{rendered_non_empty_lines, test_assets, test_md_theme};
use crate::markdown::{line_plain_text, parse_markdown};

#[test]
fn simple_reference_and_definition_render_superscript_and_notes_block() {
    let (ss, theme) = test_assets();
    let src = "Body with a ref[^1] here.\n\n[^1]: A footnote definition.\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let body_idx = rendered
        .iter()
        .position(|line| line.contains("Body with a ref"))
        .expect("body line missing");
    assert!(
        rendered[body_idx].contains('¹'),
        "expected superscript ¹ in body, got {:?}",
        rendered[body_idx]
    );

    let notes_idx = rendered
        .iter()
        .position(|line| line == "Notes")
        .expect("expected `Notes` header line");
    assert!(notes_idx > body_idx);
    let def_line = rendered
        .iter()
        .find(|line| line.contains("A footnote definition."))
        .expect("expected the definition text in the notes block");
    assert!(
        def_line.contains('¹'),
        "expected superscript ¹ on definition line, got {def_line:?}"
    );
}

#[test]
fn document_without_footnotes_emits_no_notes_block() {
    let (ss, theme) = test_assets();
    let src = "Just some content.\n\nAnother paragraph.\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered: Vec<String> = lines.iter().map(line_plain_text).collect();

    assert!(
        rendered.iter().all(|line| line != "Notes"),
        "no `Notes` header expected on a plain document, got {rendered:?}"
    );
    assert!(
        rendered.iter().all(|line| !line.contains('¹')),
        "no superscript should appear in a plain document, got {rendered:?}"
    );
}

#[test]
fn definition_before_reference_still_works() {
    let (ss, theme) = test_assets();
    let src = "[^a]: The note body.\n\nThe body references[^a] the note.\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let body = rendered
        .iter()
        .find(|line| line.contains("The body references"))
        .expect("body line missing");
    assert!(
        body.contains('¹'),
        "expected superscript ¹ after `references`, got {body:?}"
    );
    let notes_idx = rendered
        .iter()
        .position(|line| line == "Notes")
        .expect("`Notes` header expected");
    let def_line = rendered
        .iter()
        .skip(notes_idx)
        .find(|line| line.contains("The note body."))
        .expect("definition text missing in notes block");
    assert!(def_line.contains('¹'));
}

#[test]
fn orphan_reference_does_not_create_notes_block_entry() {
    let (ss, theme) = test_assets();
    // Reference with no matching definition — no `Notes` block should appear.
    let src = "Here is an orphan[^missing] reference.\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let body = rendered
        .iter()
        .find(|line| line.contains("Here is an orphan"))
        .expect("body line missing");
    assert!(
        body.contains('¹'),
        "orphan reference should still render superscript, got {body:?}"
    );
    assert!(
        rendered.iter().all(|line| line != "Notes"),
        "no `Notes` header when only orphan references are present, got {rendered:?}"
    );
}

#[test]
fn definition_body_spans_use_footnote_text_color() {
    use ratatui::style::Color;
    let (ss, theme) = test_assets();
    let mut md_theme = test_md_theme();
    md_theme.text = Color::Rgb(1, 2, 3);
    md_theme.footnote_text = Color::Rgb(200, 100, 50);
    let src = "Body[^n].\n\n[^n]: Note content here.\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &md_theme, false, true).into();

    let def_line = lines
        .iter()
        .find(|line| line_plain_text(line).contains("Note content here."))
        .expect("definition line missing");
    let content_span = def_line
        .spans
        .iter()
        .find(|s| s.content.contains("Note content here."))
        .expect("content span missing");
    assert_eq!(
        content_span.style.fg,
        Some(md_theme.footnote_text),
        "definition body span should be recolored to footnote_text"
    );

    let body_line = lines
        .iter()
        .find(|line| line_plain_text(line).contains("Body"))
        .expect("body line missing");
    let body_span = body_line
        .spans
        .iter()
        .find(|s| s.content.starts_with("Body"))
        .expect("body content span missing");
    assert_eq!(
        body_span.style.fg,
        Some(md_theme.text),
        "body prose should keep the regular text color"
    );
}

#[test]
fn unreferenced_definition_appears_in_notes_after_referenced_ones() {
    let (ss, theme) = test_assets();
    let src = "First ref[^one].\n\n[^one]: First def.\n\n[^two]: Second def (orphan).\n";
    let (lines, _, _, _) = parse_markdown(src, &ss, &theme, &test_md_theme(), false, true).into();
    let rendered = rendered_non_empty_lines(&lines);

    let first_idx = rendered
        .iter()
        .position(|line| line.contains("First def."))
        .expect("first definition missing");
    let second_idx = rendered
        .iter()
        .position(|line| line.contains("Second def (orphan)."))
        .expect("orphan definition missing");
    assert!(first_idx < second_idx);
    let second_line = &rendered[second_idx];
    assert!(
        second_line.contains('²'),
        "orphan definition should carry superscript ², got {second_line:?}"
    );
}
