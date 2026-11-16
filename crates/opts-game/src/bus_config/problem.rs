//! Describing a problem of a bus configuration: its wording, and the Display of each problem.

use super::*;

impl Problem {
    /// A short stable name for protocol clients.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::VehicleMissing { .. } => "vehicle_missing",
            Self::RepaintMissing { .. } => "repaint_missing",
            Self::SetvarUnknown { .. } => "setvar_unknown",
            Self::SetvarRejected { .. } => "setvar_rejected",
        }
    }

    /// Whether the configuration can still be loaded (with this part left out).
    #[must_use]
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::VehicleMissing { .. })
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VehicleMissing { vehicle } => {
                write!(f, "the vehicle `{vehicle}` is not installed")
            }
            Self::RepaintMissing { repaint } => write!(
                f,
                "the repaint `{repaint}` is not offered by the vehicle; the default is used"
            ),
            Self::SetvarUnknown { name } => {
                write!(f, "the vehicle has no option `{name}`; it is not applied")
            }
            Self::SetvarRejected { name, value } => write!(
                f,
                "the value {value} is not accepted for the option `{name}`; it is not applied"
            ),
        }
    }
}
