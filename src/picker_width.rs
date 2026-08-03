#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickerWidthSpec {
    Percent(u8),
    Cells(u16),
}

pub(crate) const DEFAULT_PICKER_WIDTH: PickerWidthSpec = PickerWidthSpec::Cells(78);
pub(crate) const PICKER_WIDTH_FLOOR_CELLS: u16 = 78;
pub(crate) const PICKER_WIDTH_FLOOR_WARNING: &str =
    "Configured file picker width below minimum, floor applied";

pub(crate) fn parse_picker_width_spec(s: &str) -> Option<PickerWidthSpec> {
    let s = s.trim().to_ascii_lowercase();
    if let Some(num) = s.strip_suffix('%') {
        let n: u8 = num.parse().ok()?;
        return (1..=100)
            .contains(&n)
            .then_some(PickerWidthSpec::Percent(n));
    }
    for suffix in ["cells", "cell"] {
        if let Some(num) = s.strip_suffix(suffix) {
            let n: u16 = num.parse().ok()?;
            return (n >= 1).then_some(PickerWidthSpec::Cells(n));
        }
    }
    None
}

pub(crate) fn picker_width_raw(spec: PickerWidthSpec, area_width: u16) -> u16 {
    match spec {
        PickerWidthSpec::Percent(p) => {
            (area_width as u32).saturating_mul(p as u32).div_ceil(100) as u16
        }
        PickerWidthSpec::Cells(n) => n,
    }
}
