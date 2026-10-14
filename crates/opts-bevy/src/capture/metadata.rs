//! The metadata object that accompanies every capture (schema in
//! `docs/src/architecture/screen-capture.md`, section A3).

use bevy::ecs::world::World;
use bevy::prelude::*;
use bevy::time::{Fixed, Virtual};
use bevy::window::PrimaryWindow;
use serde_json::{json, Map, Value};

use crate::control::state::ControlState;
use crate::reporting::state::ReportContext;

use super::storage::CaptureState;

/// Version of the metadata object; bumped when a key changes meaning.
pub const SCHEMA: u32 = 1;

/// A function that adds game-specific keys (map, tile, application state) to the metadata.
pub type MetadataHook = fn(&World, &mut Map<String, Value>);

/// Extra metadata providers; `opts-game` registers the ones that need game resources.
#[derive(Resource, Default)]
pub struct CaptureMetaHooks(pub Vec<MetadataHook>);

/// What a metadata object describes.
#[derive(Debug, Clone, Copy)]
pub struct Subject<'a> {
    /// `screenshot`, `burst_frame`, `recording` …
    pub kind: &'a str,
    /// File name (not the full path: the user's directory layout stays out of the file).
    pub file: &'a str,
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// File format (`png`, `jpeg`, `av1`).
    pub format: &'a str,
}

/// Collects the metadata of the world as it is now.
///
/// Simulation time is `Time<Virtual>` (it stands still while paused), `frame` is the control
/// server's render-frame counter, the same number `state` reports.
#[must_use]
pub fn collect(world: &mut World, subject: Subject<'_>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("schema".into(), json!(SCHEMA));
    m.insert("kind".into(), json!(subject.kind));
    m.insert("file".into(), json!(subject.file));
    m.insert(
        "created".into(),
        json!(chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false)),
    );
    m.insert("width".into(), json!(subject.width));
    m.insert("height".into(), json!(subject.height));
    m.insert("format".into(), json!(subject.format));
    if let Some(v) = world.get_resource::<Time<Virtual>>() {
        m.insert(
            "sim_time".into(),
            json!((v.elapsed_secs_f64() * 1000.0).round() / 1000.0),
        );
        m.insert("sim_paused".into(), json!(v.is_paused()));
    }
    if let Some(f) = world.get_resource::<Time<Fixed>>() {
        let step = f.timestep().as_nanos().max(1);
        m.insert("fixed_tick".into(), json!(f.elapsed().as_nanos() / step));
    }
    if let Some(c) = world.get_resource::<ControlState>() {
        m.insert("frame".into(), json!(c.frame));
        m.insert(
            "fps".into(),
            json!((f64::from(c.fps) * 10.0).round() / 10.0),
        );
    }
    let ctx = world.get_resource::<ReportContext>().cloned();
    if let Some(ctx) = ctx {
        for (key, val) in [
            ("map", ctx.map),
            ("camera_mode", ctx.camera),
            ("vehicle", ctx.vehicle),
            ("git", ctx.git),
        ] {
            if let Some(v) = val {
                m.insert(key.into(), json!(v));
            }
        }
    }
    m.insert("app_version".into(), json!(env!("CARGO_PKG_VERSION")));
    let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
    if let Some(w) = windows.iter(world).next() {
        m.insert(
            "window".into(),
            json!({
                "mode": format!("{:?}", w.mode),
                "scale_factor": w.resolution.scale_factor(),
                "physical": [w.resolution.physical_width(), w.resolution.physical_height()],
            }),
        );
    }
    if let Some(s) = world.get_resource::<CaptureState>() {
        m.insert("session_id".into(), json!(s.session_id));
    }
    if let Some(hooks) = world.get_resource::<CaptureMetaHooks>() {
        let hooks = hooks.0.clone();
        for hook in hooks {
            hook(world, &mut m);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_carries_sim_time_frame_and_hook_keys() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().init_resource::<ControlState>();
        app.world_mut().resource_mut::<ControlState>().frame = 42;
        app.world_mut()
            .insert_resource(CaptureMetaHooks(vec![|_, m| {
                m.insert("tile".into(), json!([3, 4]));
            }]));
        app.update();
        let m = collect(
            app.world_mut(),
            Subject {
                kind: "screenshot",
                file: "a.png",
                width: 64,
                height: 48,
                format: "png",
            },
        );
        assert_eq!(m["schema"], 1);
        assert_eq!(m["frame"], 42);
        assert!(m["sim_time"].is_number(), "{m:?}");
        assert_eq!(m["width"], 64);
        assert_eq!(m["tile"], json!([3, 4]));
        assert!(!m.contains_key("hardware"), "no hardware data, ever");
    }
}
