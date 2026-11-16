//! Unit tests for `super` (`lane.rs`): the lane fixtures, and the spline and object lanes in their own files.

use super::*;
use glam::Vec3;
use opts_core::formats::scenery::SplinePath;

mod object;
mod spline;

fn spline(heading: f32, radius: f32, length: f32) -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: 1,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::new(10.0, 20.0, 5.0),
        heading_deg: heading,
        length_m: length,
        radius_m: radius,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: Vec::new(),
    }
}

fn profile(offset: f32) -> SplineProfile {
    SplineProfile {
        paths: vec![
            SplinePath {
                path_type: 0,
                offset_x: -2.5,
                offset_z: 0.1,
                width: 3.0,
                direction: 0,
                extra: None,
            },
            SplinePath {
                path_type: 1,
                offset_x: offset,
                offset_z: 0.05,
                width: 0.6,
                direction: 2,
                extra: None,
            },
        ],
        ..Default::default()
    }
}
