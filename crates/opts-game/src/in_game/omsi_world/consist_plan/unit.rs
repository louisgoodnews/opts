//! One unit of a consist (a car and its `[couple_back]` chain): walked along the links and placed
//! into the train's axis. The frames are described in the parent module.

use std::path::{Path, PathBuf};

use bevy::prelude::Vec3;
use opts_core::formats::vehicles::BusManifest;

use super::super::vehicle::coupling_joint;
use super::{Files, PlannedPart};

/// A part in a unit's own frame.
#[derive(Debug, Clone)]
pub(super) struct Local {
    pub(super) path: PathBuf,
    pub(super) dir: PathBuf,
    pub(super) manifest: BusManifest,
    pub(super) turned: bool,
    pub(super) behind: f32,
    pub(super) joint: Option<(Vec3, Vec3)>,
}


/// The unit that starts with `first`: follows `[couple_back]` links (a link flagged reversed ends
/// the unit: the chain leads back into itself, as the NF6D's rear cab does), at most `budget`
/// files.
pub(super) fn walk(first: Local, files: &dyn Files, budget: usize) -> Vec<Local> {
    let mut parts = vec![first];
    while parts.len() < budget {
        let Some(parent) = parts.last() else { break };
        let Some(name) = parent.manifest.couple_back.clone() else {
            break;
        };
        let Some(path) = files.find_in(&parent.dir, &name) else {
            tracing::error!(trailer = %name, "trailer .bus not found; vehicle stays without it");
            break;
        };
        let Some(manifest) = files.manifest(&path) else {
            tracing::error!(trailer = %path.display(), "cannot read the trailer");
            break;
        };
        let link_reversed = parent.manifest.couple_back_reversed;
        let turned = parent.turned != link_reversed;
        let Some((b, f)) = coupling_joint(&parent.manifest, parent.turned, &manifest, turned)
        else {
            tracing::error!(trailer = %path.display(), "coupling points missing; trailer not attached");
            break;
        };
        let behind = parent.behind + (f.y - b.y);
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        parts.push(Local {
            path,
            dir,
            manifest,
            turned,
            behind,
            joint: Some((b, f)),
        });
        if link_reversed {
            break;
        }
    }
    parts
}

/// The unit's front and rear coupling points on the unit's axis (local y).
fn ends(unit: &[Local]) -> (f32, f32) {
    let front = unit
        .first()
        .and_then(|p| p.manifest.coupling_front)
        .map_or(0.0, |c| c.y);
    let rear = unit.last().map_or(0.0, |p| {
        let own = if p.turned {
            p.manifest.coupling_front.map(|c| -c.y)
        } else {
            p.manifest.coupling_back.map(|c| c.y)
        };
        -p.behind + own.unwrap_or(0.0)
    });
    (front, rear)
}

/// Puts a unit into the train: returns its parts in train coordinates and the train y of its
/// rear coupling point.
pub(super) fn place(
    unit: Vec<Local>,
    turned: bool,
    prev_rear: Option<f32>,
    powered: bool,
) -> (Vec<PlannedPart>, f32) {
    let (front, rear) = ends(&unit);
    // `y_train = t + y_local` (or `t - y_local` turned); the unit's front joint meets the rear
    // joint of the unit before it.
    let t = match (prev_rear, turned) {
        (None, _) => 0.0,
        (Some(p), false) => p - front,
        (Some(p), true) => p + rear,
    };
    let rear_train = if turned { t - front } else { t + rear };
    let parts = unit
        .into_iter()
        .map(|p| PlannedPart {
            behind: if turned {
                -(t + p.behind)
            } else {
                p.behind - t
            },
            turned: p.turned != turned,
            path: p.path,
            dir: p.dir,
            manifest: p.manifest,
            powered,
            joint: p.joint,
        })
        .collect();
    (parts, rear_train)
}
