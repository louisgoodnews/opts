//! Import and export directions of a pack.

/// Writing direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Left to right.
    #[default]
    Ltr,
    /// Right to left (not supported by the interface yet).
    Rtl,
}

impl Direction {
    /// `ltr` or `rtl`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}
