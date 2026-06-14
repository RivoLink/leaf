#[derive(Clone)]
pub(crate) struct TocEntry {
    pub(crate) level: u8,
    pub(crate) title: String,
    pub(crate) line: usize,
}

/// Computes how many levels to shift headings down so that the document's
/// top-level headings appear as root (display level 1).
///
/// Returns `(shift, hide_root)` where:
/// - `shift` is the number of levels to subtract from every heading
/// - `hide_root` indicates the single top-level heading should be hidden
///   (analogous to the old "hide single h1" behaviour)
pub(crate) fn toc_promotion(toc: &[TocEntry]) -> (u8, bool) {
    let Some(min_level) = toc.iter().map(|e| e.level).min() else {
        return (0, false);
    };

    let min_count = toc.iter().filter(|e| e.level == min_level).count();
    let has_deeper = toc.iter().any(|e| e.level > min_level);

    // Only hide the single root heading when it is an H1; for other
    // minimum levels we simply shift without hiding.
    if min_level == 1 && min_count == 1 && has_deeper {
        let next_min = toc
            .iter()
            .filter(|e| e.level > min_level)
            .map(|e| e.level)
            .min()
            .unwrap_or(min_level);
        (next_min - 1, true)
    } else {
        (min_level - 1, false)
    }
}

pub(crate) fn toc_display_level(level: u8, shift: u8, hide_root: bool) -> u8 {
    if hide_root && level <= shift {
        // This entry is the hidden single root; callers filter it out,
        // but if it reaches here return 0 as a sentinel.
        return 0;
    }
    level.saturating_sub(shift)
}

pub(crate) fn normalize_toc(mut toc: Vec<TocEntry>) -> Vec<TocEntry> {
    let Some(min_level) = toc.iter().map(|e| e.level).min() else {
        return toc;
    };
    // Keep at most 3 heading levels starting from the document minimum.
    let max_raw = min_level + 2;
    toc.retain(|entry| entry.level <= max_raw);
    toc
}
