use ratatui::layout::Size;
use ratatui_image::{
    picker::{Picker, ProtocolType},
    sliced::SlicedProtocol,
    Resize,
};
use std::path::{Path, PathBuf};

/// A decoded, terminal-ready image placed at `rendered_start` in the parsed line buffer.
pub(crate) struct ImageEntry {
    pub(crate) rendered_start: usize,
    pub(crate) slice: SlicedProtocol,
}

fn is_remote_url(dest: &str) -> bool {
    dest.starts_with("http://") || dest.starts_with("https://") || dest.starts_with("data:")
}

fn resolve_image_path(dest: &str, base_path: Option<&Path>) -> PathBuf {
    let path = Path::new(dest);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match base_path {
        Some(base) => base.join(path),
        None => path.to_path_buf(),
    }
}

/// Decodes a local image and slices it to fit within `render_width` columns, preserving
/// its aspect ratio via the terminal's font-cell size. Returns `None` for remote URLs,
/// images that can't be read/decoded, or when the terminal has no real graphics protocol
/// (halfblock art isn't worth the noise), in which case callers should fall back to a
/// text placeholder.
pub(crate) fn load_image(
    dest: &str,
    base_path: Option<&Path>,
    render_width: usize,
    picker: &Picker,
) -> Option<(SlicedProtocol, usize)> {
    if picker.protocol_type() == ProtocolType::Halfblocks {
        return None;
    }
    if is_remote_url(dest) {
        return None;
    }
    let path = resolve_image_path(dest, base_path);
    let dyn_img = image::ImageReader::open(&path).ok()?.decode().ok()?;

    let font_size = picker.font_size();
    // Unbounded height: images are allowed to run tall and simply scroll, like code blocks.
    let bounds = Size::new(render_width.max(1) as u16, u16::MAX);
    let size = Resize::Fit(None).size_for(&dyn_img, font_size, bounds);
    let slice = SlicedProtocol::new_with_resize(picker, dyn_img, size, Resize::Fit(None)).ok()?;
    let height = (slice.size().height as usize).max(1);
    Some((slice, height))
}
