//! Unit tests of `source.rs`, split by concern: road lanes, traffic rules and object paths.

mod objects;
mod roads;
mod rules;

use glam::{DVec2, Vec3};
use opts_core::formats::map::{MapObjectInstance, MapSplineInstance, MapTile, MapTrafficRule, RuleOwner, TileGrid};
use opts_core::formats::scenery::{ScoPath, SplinePath};
use opts_traffic::graph::{ConnectParams, LaneGraph};
use opts_traffic::source::{segments_from_tile, tile_origin, PathSource};

struct Fixed {
    sli: Vec<SplinePath>,
    sco: Vec<ScoPath>,
}

impl PathSource for Fixed {
    fn sli_paths(&mut self, _: &str) -> Option<&[SplinePath]> {
        Some(&self.sli)
    }
    fn sco_paths(&mut self, _: &str) -> Option<&[ScoPath]> {
        Some(&self.sco)
    }
}

fn spline_path(offset_x: f32, direction: u32) -> SplinePath {
    SplinePath {
        path_type: 0,
        offset_x,
        offset_z: 0.0,
        width: 2.5,
        direction,
        extra: None,
    }
}

fn spline(y: f32, len: f32) -> MapSplineInstance {
    MapSplineInstance {
        flag: 0,
        sli_file: "road.sli".into(),
        id_code: 7,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::new(0.0, y, 0.0),
        heading_deg: 0.0,
        length_m: len,
        radius_m: 0.0,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: Vec::new(),
    }
}

fn sco_path(x: f32, y: f32) -> ScoPath {
    ScoPath {
        start: Vec3::new(x, y, 0.0),
        heading_deg: 0.0,
        radius_m: -999.99,
        length_m: 20.0,
        slope_start: 0.0,
        slope_end: 0.0,
        path_type: 0,
        width_m: 2.5,
        direction: 0,
        extra: None,
        traffic_light_index: None,
        approach_distance_m: None,
        switch_position: None,
    }
}

/// End of a placed spline's centre line by the mesh formula (`opts_bevy::scenery::spline_pose`).
fn mesh_end(s: &MapSplineInstance) -> DVec2 {
    let (r, l) = (f64::from(s.radius_m), f64::from(s.length_m));
    let phi = f64::from(s.heading_deg).to_radians();
    let h = phi + l / r;
    DVec2::new(
        f64::from(s.position.x) - r * (h.cos() - phi.cos()),
        f64::from(s.position.y) + r * (h.sin() - phi.sin()),
    )
}
