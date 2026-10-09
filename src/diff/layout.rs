#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DiffLayout {
    #[default]
    Unified,
    Split,
}

impl DiffLayout {
    pub(crate) fn toggled(self) -> Self {
        match self {
            DiffLayout::Unified => DiffLayout::Split,
            DiffLayout::Split => DiffLayout::Unified,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggled_flips_between_the_two_variants() {
        assert_eq!(DiffLayout::Unified.toggled(), DiffLayout::Split);
        assert_eq!(DiffLayout::Split.toggled(), DiffLayout::Unified);
    }

    #[test]
    fn default_is_unified() {
        assert_eq!(DiffLayout::default(), DiffLayout::Unified);
    }
}
