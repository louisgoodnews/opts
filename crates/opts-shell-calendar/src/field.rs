//! The parts of a date or time that the steppers change, with their ids, labels and widths.

/// Supported years.
pub const YEARS: std::ops::RangeInclusive<i32> = 1900..=2100;

/// A part of the date or time the steppers change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    /// Day of the month.
    Day,
    /// Month.
    Month,
    /// Year.
    Year,
    /// Hour.
    Hour,
    /// Minute.
    Minute,
}

impl Field {
    /// The date fields in display order.
    pub const DATE: [Self; 3] = [Self::Day, Self::Month, Self::Year];
    /// The time fields in display order.
    pub const TIME: [Self; 2] = [Self::Hour, Self::Minute];

    /// Stable id fragment (`day`, `month`, ...), used in `UiId`s.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Month => "month",
            Self::Year => "year",
            Self::Hour => "hour",
            Self::Minute => "minute",
        }
    }

    /// Message key of the stepper label.
    pub const fn label_key(self) -> &'static str {
        match self {
            Self::Day => "wiz-when-day",
            Self::Month => "wiz-when-month",
            Self::Year => "wiz-when-year",
            Self::Hour => "wiz-when-hour",
            Self::Minute => "wiz-when-minute",
        }
    }

    /// Width of the value box.
    pub const fn value_width(self) -> f32 {
        match self {
            Self::Day | Self::Hour | Self::Minute => 48.0,
            Self::Month => 96.0,
            Self::Year => 64.0,
        }
    }
}
