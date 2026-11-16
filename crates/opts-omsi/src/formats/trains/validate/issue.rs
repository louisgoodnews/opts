//! The reasons a formation cannot be spawned ([`FormationIssue`]) and the end of a car a
//! missing coupling is at ([`End`]), with their stable keys and messages.

use std::fmt;

use super::MAX_PARTS;

/// Which end of a car a coupling is missing at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// Towards the front of the train.
    Front,
    /// Towards the rear of the train.
    Rear,
}

/// Why a formation cannot be spawned. `index` is the 0-based car.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormationIssue {
    /// Fewer than two cars (V1).
    TooShort,
    /// More than [`MAX_PARTS`] files in total (V2).
    TooManyParts {
        /// Parts counted.
        parts: usize,
    },
    /// The vehicle file does not exist (V3).
    Missing {
        /// Car.
        index: usize,
        /// File as written.
        file: String,
    },
    /// The vehicle is not a rail vehicle (V4).
    NotRail {
        /// Car.
        index: usize,
    },
    /// The first car has no driver workstation (V5).
    NoCab,
    /// A car lacks the coupling point towards a neighbour (V6).
    Coupling {
        /// Car.
        index: usize,
        /// The end of the train it faces.
        end: End,
    },
    /// The entry names a vehicle group, which needs a map (V7).
    Group {
        /// Car.
        index: usize,
    },
    /// The first car is turned around (V9): the player drives it, so it must face forward.
    HeadTurned,
}

impl FormationIssue {
    /// A stable key for messages (`formation-err-...`).
    #[must_use]
    pub fn key(&self) -> &'static str {
        match self {
            Self::TooShort => "formation-err-too-short",
            Self::TooManyParts { .. } => "formation-err-too-many-parts",
            Self::Missing { .. } => "formation-err-missing",
            Self::NotRail { .. } => "formation-err-not-rail",
            Self::NoCab => "formation-err-no-cab",
            Self::Coupling { .. } => "formation-err-coupling",
            Self::Group { .. } => "formation-err-group",
            Self::HeadTurned => "formation-err-head-turned",
        }
    }

    /// The car the issue is about, if any (0-based).
    #[must_use]
    pub fn car(&self) -> Option<usize> {
        match self {
            Self::Missing { index, .. }
            | Self::NotRail { index }
            | Self::Coupling { index, .. }
            | Self::Group { index } => Some(*index),
            Self::NoCab | Self::HeadTurned => Some(0),
            Self::TooShort | Self::TooManyParts { .. } => None,
        }
    }
}

impl fmt::Display for FormationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "a formation needs at least two cars"),
            Self::TooManyParts { parts } => {
                write!(f, "{parts} vehicle files in total; at most {MAX_PARTS}")
            }
            Self::Missing { index, file } => write!(f, "car {}: `{file}` not found", index + 1),
            Self::NotRail { index } => write!(f, "car {}: not a rail vehicle", index + 1),
            Self::NoCab => write!(f, "the first car has no driver workstation"),
            Self::Coupling { index, end } => write!(
                f,
                "car {}: no coupling point towards the {} of the train",
                index + 1,
                match end {
                    End::Front => "front",
                    End::Rear => "rear",
                }
            ),
            Self::Group { index } => write!(
                f,
                "car {}: a vehicle group name cannot be spawned outside a map's AI lists",
                index + 1
            ),
            Self::HeadTurned => write!(f, "the first car must face forward (use Reverse train)"),
        }
    }
}
