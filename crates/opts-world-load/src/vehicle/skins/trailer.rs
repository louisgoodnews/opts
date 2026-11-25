//! The trailer link of an articulated head: the coupling pair its bellow's trailer end follows.

use bevy::math::Vec3;
use opts_core::formats::vehicles::BusManifest;
use std::path::Path;

use super::super::super::WorldLoadJob;
use super::super::coupling_joint;

/// The first coupled trailer of a head: the head's coupling point and the trailer's front point,
/// in the same OMSI frame (the pair of [`coupling_joint`]). The trailer hitch is `coupling`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TrailerLink {
    pub(crate) coupling: Vec3,
    pub(crate) front: Vec3,
}

/// The trailer coupled behind `head` (its `[couple_back]` file, found in `bus_dir`). `None` when
/// the head has no coupled trailer, the link is reversed (the chain leads back into the unit),
/// the head is a rail unit, or the trailer's points are missing.
pub(crate) fn trailer_link(
    job: &WorldLoadJob,
    bus_dir: &Path,
    head: &BusManifest,
) -> Option<TrailerLink> {
    if super::super::super::is_rail(head) || head.couple_back_reversed {
        return None;
    }
    let name = head.couple_back.as_ref()?;
    let path = job.install.find_in(bus_dir, name)?;
    let trailer = BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()?;
    let (coupling, front) = coupling_joint(head, false, &trailer, false)?;
    Some(TrailerLink { coupling, front })
}
