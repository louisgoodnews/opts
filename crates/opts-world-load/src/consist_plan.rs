//! Where every file of a consist goes: the head's `[couple_back]` chain and the cars of a train
//! formation, each car with its own chain, placed along the train's axis.
//!
//! Pure geometry over manifests (no entities, no models), so the placement is tested without a
//! session. The loader (`vehicle::spawn_trailer_chain`) spawns what the plan lists.
//!
//! Frames: the train's y axis points forward and its origin is the head's origin; `behind` is the
//! distance of a part's origin behind it. Inside a unit (a car and its chain) the same axis is used
//! with the unit's first file at 0 ("local"). A unit turned around is the local layout mirrored:
//! `y_train = t - y_local` instead of `t + y_local`, every part turned, the order reversed.

use std::path::{Path, PathBuf};

use bevy::prelude::Vec3;
use opts_core::formats::vehicles::BusManifest;

use super::vehicle::FormationCar;

mod unit;
use unit::{place, walk, Local};

/// Reads the installation: files named relative to a folder, and manifests.
pub(super) trait Files {
    /// The file `rel` relative to `base` (case-insensitive, backslashes).
    fn find_in(&self, base: &Path, rel: &str) -> Option<PathBuf>;
    /// The parsed manifest of a vehicle file.
    fn manifest(&self, path: &Path) -> Option<BusManifest>;
}

/// One file of the consist behind the head.
#[derive(Debug, Clone)]
pub(super) struct PlannedPart {
    pub path: PathBuf,
    pub dir: PathBuf,
    pub manifest: BusManifest,
    /// Runs turned around in the train.
    pub turned: bool,
    /// Distance of its origin behind the head's origin (m).
    pub behind: f32,
    /// Its unit pulls.
    pub powered: bool,
    /// The coupling points of the link to the part in front inside its unit (rear point of the
    /// leader, front point of this part); `None` for the first file of a unit.
    pub joint: Option<(Vec3, Vec3)>,
}

/// The plan: the files behind the head, front to back.
#[derive(Debug, Default)]
pub(super) struct Plan {
    pub followers: Vec<PlannedPart>,
}

/// Plans the files behind the head: its own chain, then each car of `cars` with its chain, at most
/// `limit` files in all (the head not counted).
pub(super) fn plan(
    head_dir: &Path,
    head: &BusManifest,
    cars: &[FormationCar],
    limit: usize,
    files: &dyn Files,
) -> Plan {
    let first = Local {
        path: PathBuf::new(),
        dir: head_dir.to_path_buf(),
        manifest: head.clone(),
        turned: false,
        behind: 0.0,
        joint: None,
    };
    let unit = walk(first, files, limit + 1);
    let (parts, mut rear) = place(unit, false, None, true);
    let mut followers: Vec<PlannedPart> = parts.into_iter().skip(1).collect();
    for car in cars {
        let left = limit.saturating_sub(followers.len());
        if left == 0 {
            break;
        }
        let Some(manifest) = files.manifest(&car.path) else {
            tracing::error!(trailer = %car.path.display(), "cannot read the car; the train ends before it");
            break;
        };
        let dir = car.path.parent().map(Path::to_path_buf).unwrap_or_default();
        let unit = walk(
            Local {
                path: car.path.clone(),
                dir,
                manifest,
                turned: false,
                behind: 0.0,
                joint: None,
            },
            files,
            left,
        );
        // A car made of several files is a multiple unit: it pulls unless told not to.
        let powered = car.powered.unwrap_or(unit.len() > 1);
        let (parts, r) = place(unit, car.turned, Some(rear), powered);
        rear = r;
        followers.extend(parts);
    }
    if !cars.is_empty() {
        // A turned unit comes out back to front; the train order is the order of the origins.
        followers.sort_by(|a, b| a.behind.total_cmp(&b.behind));
    }
    Plan { followers }
}

#[cfg(test)]
mod tests;
