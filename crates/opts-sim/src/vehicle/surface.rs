//! The host's surface state of a vehicle (M-0120, M-0121): the dirt level the scripts read as
//! `Dirt_Norm`, the wetness of the vehicle's paint and the last wash. Nothing in the stock scripts
//! computes the dirt, so the host sets it (`vehicle.surface`) and the script system feeds it.

use serde::{Deserialize, Serialize};

/// The surface state of one vehicle. All values are shares from 0 to 1 (the range is checked by
/// the control protocol, not here).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct VehicleSurface {
    /// Dirt level `0..=1`, the value of the script variable `Dirt_Norm`.
    pub dirt: f32,
    /// Wetness of the paint and the surfaces `0..=1`.
    pub wet: f32,
    /// Session clock second of the last wash; `None` before the first one.
    #[serde(default)]
    pub wiped_at: Option<f32>,
}

impl VehicleSurface {
    /// A wash: the dirt goes to 0 and the wash is remembered with its clock second `at`.
    pub fn wash(&mut self, at: f32) {
        self.dirt = 0.0;
        self.wiped_at = Some(at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wash_clears_the_dirt_and_remembers_its_time() {
        let mut s = VehicleSurface {
            dirt: 0.8,
            ..VehicleSurface::default()
        };
        assert_eq!(s.wiped_at, None);
        s.wash(1234.0);
        assert_eq!(s.dirt, 0.0);
        assert_eq!(s.wiped_at, Some(1234.0));
    }
}
