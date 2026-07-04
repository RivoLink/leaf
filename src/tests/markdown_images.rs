use super::{test_assets, test_md_theme};
use crate::markdown::parse_markdown_with_width;
use image::{Rgb, RgbImage};
use ratatui_image::picker::{Picker, ProtocolType};
use std::path::PathBuf;

fn picker_with_protocol(protocol: ProtocolType) -> Picker {
    let mut picker = Picker::halfblocks();
    picker.set_protocol_type(protocol);
    picker
}

fn write_test_png(name: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "leaf-image-test-{}-{}",
        std::process::id(),
        name.replace('.', "_")
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    RgbImage::from_pixel(20, 20, Rgb([200, 50, 50]))
        .save(&path)
        .expect("write test png");
    (dir, path)
}

#[test]
fn local_image_is_decoded_and_reserves_lines() {
    let (ss, theme) = test_assets();
    let (dir, _path) = write_test_png("pic.png");

    let src = "![a pic](pic.png)\n";
    let picker = picker_with_protocol(ProtocolType::Kitty);
    let parsed = parse_markdown_with_width(
        src,
        &ss,
        &theme,
        40,
        &test_md_theme(),
        false,
        true,
        Some(dir.as_path()),
        &picker,
    );

    assert_eq!(parsed.images.len(), 1);
    let entry = &parsed.images[0];
    let height = entry.slice.size().height as usize;
    assert!(height > 0);
    assert!(parsed.lines.len() >= entry.rendered_start + height);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn halfblocks_only_terminal_falls_back_to_placeholder() {
    let (ss, theme) = test_assets();
    let (dir, _path) = write_test_png("pic.png");

    let src = "![a pic](pic.png)\n";
    let picker = Picker::halfblocks();
    let parsed = parse_markdown_with_width(
        src,
        &ss,
        &theme,
        40,
        &test_md_theme(),
        false,
        true,
        Some(dir.as_path()),
        &picker,
    );

    assert!(parsed.images.is_empty());
    let text: String = parsed
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.content.as_ref())
        .collect();
    assert!(text.contains("[img: a pic]"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn missing_image_falls_back_to_placeholder() {
    let (ss, theme) = test_assets();
    let src = "![missing alt](does-not-exist.png)\n";
    let picker = picker_with_protocol(ProtocolType::Kitty);
    let parsed = parse_markdown_with_width(
        src,
        &ss,
        &theme,
        40,
        &test_md_theme(),
        false,
        true,
        None,
        &picker,
    );

    assert!(parsed.images.is_empty());
    let text: String = parsed
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.content.as_ref())
        .collect();
    assert!(text.contains("[img: missing alt]"));
}

#[test]
fn remote_image_url_falls_back_to_placeholder_without_fetching() {
    let (ss, theme) = test_assets();
    let src = "![remote](https://example.com/a.png)\n";
    let picker = picker_with_protocol(ProtocolType::Kitty);
    let parsed = parse_markdown_with_width(
        src,
        &ss,
        &theme,
        40,
        &test_md_theme(),
        false,
        true,
        None,
        &picker,
    );

    assert!(parsed.images.is_empty());
    let text: String = parsed
        .lines
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.content.as_ref())
        .collect();
    assert!(text.contains("[img: remote]"));
}
