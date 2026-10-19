//! Validation of a formation against the vehicles it names (design: `train-formation-editor.md`
//! section 6). The checks are pure: what is known about each vehicle comes through [`VehicleLookup`],
//! which `opts-game` implements with the installation index and the manifests.

use std::fmt;

use super::{Orientation, TrainFormation, VehicleRef};
use crate::formats::vehicles::BusManifest;

/// The most parts (vehicle files, bogies and joints included) one formation may have; the walker
/// of a single vehicle stops at 40.
pub const MAX_PARTS: usize = 64;

/// What the validator needs to know about one car's vehicle file.
#[derive(Debug, Clone, PartialEq)]
pub struct VehicleInfo {
    /// `[type]` 2 with `[boogies]` (the rail test of `ai_profile`).
    pub is_rail: bool,
    /// A driver workstation: a driver camera.
    pub has_cab: bool,
    /// The head file has `[coupling_front]`.
    pub front_coupling: bool,
    /// The end of the car's own chain (the file itself when it has no `[couple_back]`) has
    /// `[coupling_back]`.
    pub back_coupling: bool,
    /// Files in the car's own `[couple_back]` chain, itself included (1 for a self-contained car);
    /// a link flagged reversed ends the chain.
    pub parts: usize,
}

impl VehicleInfo {
    /// The facts of a self-contained car from its manifest (`parts` is 1; a caller that walked
    /// the `[couple_back]` chain overwrites `parts` and `back_coupling`).
    #[must_use]
    pub fn from_manifest(m: &BusManifest) -> Self {
        Self {
            is_rail: m.vehicle_type == Some(2) && m.bogie_half_span.is_some(),
            has_cab: !m.driver_cameras.is_empty(),
            front_coupling: m.coupling_front.is_some(),
            back_coupling: m.coupling_back.is_some(),
            parts: 1,
        }
    }

    /// Whether the car is a chain of several files.
    #[must_use]
    pub fn is_chain(&self) -> bool {
        self.parts > 1
    }
}

/// Knows the vehicles of an installation.
pub trait VehicleLookup {
    /// The facts of the vehicle file `file` (a path as written in a `.zug`), or `None` when the
    /// file does not exist or cannot be read.
    fn info(&self, file: &str) -> Option<VehicleInfo>;
}

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

impl TrainFormation {
    /// Checks the formation; every problem found is returned (never only the first). The checks of
    /// a car whose file is missing or is no rail vehicle stop at that car.
    #[must_use]
    pub fn validate(&self, lookup: &dyn VehicleLookup) -> Vec<FormationIssue> {
        let mut issues = Vec::new();
        let n = self.segments.len();
        if n < 2 {
            issues.push(FormationIssue::TooShort);
        }
        let mut parts = 0usize;
        for (i, seg) in self.segments.iter().enumerate() {
            let file = match &seg.vehicle {
                VehicleRef::File(f) => f,
                VehicleRef::Group(_) => {
                    issues.push(FormationIssue::Group { index: i });
                    continue;
                }
            };
            let Some(info) = lookup.info(file) else {
                issues.push(FormationIssue::Missing {
                    index: i,
                    file: file.clone(),
                });
                continue;
            };
            parts += info.parts;
            if !info.is_rail {
                issues.push(FormationIssue::NotRail { index: i });
                continue;
            }
            let turned = seg.orientation == Orientation::Backward;
            if i == 0 {
                if !info.has_cab {
                    issues.push(FormationIssue::NoCab);
                }
                if turned {
                    issues.push(FormationIssue::HeadTurned);
                }
            }
            // A car needs the point on the side that faces a neighbour; turning it swaps which of
            // its own points faces which end of the train.
            let (toward_front, toward_rear) = if turned {
                (info.back_coupling, info.front_coupling)
            } else {
                (info.front_coupling, info.back_coupling)
            };
            if i > 0 && !toward_front {
                issues.push(FormationIssue::Coupling {
                    index: i,
                    end: End::Front,
                });
            }
            if i + 1 < n && !toward_rear {
                issues.push(FormationIssue::Coupling {
                    index: i,
                    end: End::Rear,
                });
            }
        }
        if parts > MAX_PARTS {
            issues.push(FormationIssue::TooManyParts { parts });
        }
        issues
    }

    /// The same train driven the other way round: the cars in the opposite order, each turned.
    #[must_use]
    pub fn reversed(&self) -> Self {
        let mut r = self.clone();
        r.segments.reverse();
        for s in &mut r.segments {
            s.orientation = s.orientation.flipped();
        }
        r
    }
}
