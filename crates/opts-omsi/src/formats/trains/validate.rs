//! Validation of a formation against the vehicles it names (design: `train-formation-editor.md`
//! section 6). The checks are pure: what is known about each vehicle comes through [`VehicleLookup`],
//! which `opts-game` implements with the installation index and the manifests.

use super::{Orientation, TrainFormation, VehicleRef};
use crate::formats::vehicles::BusManifest;

mod issue;

pub use issue::{End, FormationIssue};

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
