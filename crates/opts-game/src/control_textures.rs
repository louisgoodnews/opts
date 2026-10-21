//! Control protocol of the script textures (M-0100): `vehicle.script_textures` writes what the
//! vehicles' scripts have drawn on their canvases (destination boards, IBIS, matrix displays,
//! the ticket printer) as PNG files into the captures directory, so the displays can be looked
//! at without a camera that sees them.

use bevy::prelude::*;
use opts_bevy::capture::{storage, CaptureState};
use opts_bevy::control::params::str_param;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::omsi_world::VehicleScriptTexture;

/// Most canvases written by one call.
const MAX_FILES: usize = 64;

/// Registers the method.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.script_textures", script_textures);
}

/// `vehicle.script_textures {prefix?}`: one PNG per canvas, named `<prefix>-<n>.png` (default
/// prefix `canvas`); answers with each file's path, size and whether anything was drawn.
fn script_textures(world: &mut World, params: &Value) -> Outcome {
    let prefix = match str_param(params, "prefix") {
        Ok(p) => p.to_owned(),
        Err(_) => "canvas".to_owned(),
    };
    let root = match world
        .get_resource::<CaptureState>()
        .and_then(|s| s.ensure_root().ok())
    {
        Some(r) => r,
        None => {
            return Outcome::err(RpcError::new(
                code::NOT_READY,
                "there is no captures directory",
            ))
        }
    };
    let handles: Vec<Handle<Image>> = {
        let mut q = world.query::<&VehicleScriptTexture>();
        let mut seen = Vec::new();
        for t in q.iter(world) {
            if !seen.iter().any(|h: &Handle<Image>| h.id() == t.image.id()) {
                seen.push(t.image.clone());
            }
        }
        seen
    };
    let images = world.resource::<Assets<Image>>();
    let mut files = Vec::new();
    for (n, handle) in handles.iter().take(MAX_FILES).enumerate() {
        let Some(image) = images.get(handle) else {
            continue;
        };
        let Some(data) = image.data.as_ref() else {
            continue;
        };
        let (w, h) = (image.width(), image.height());
        let drawn = data.chunks(4).any(|p| p[3] != 0);
        let Some(buffer) = image::RgbaImage::from_raw(w, h, data.clone()) else {
            continue;
        };
        let mut png = std::io::Cursor::new(Vec::new());
        if buffer.write_to(&mut png, image::ImageFormat::Png).is_err() {
            continue;
        }
        let path = match storage::resolve_name(&root, &format!("{prefix}-{n}.png")) {
            Ok(p) => p,
            Err(e) => return Outcome::err(RpcError::invalid_params(e.to_string())),
        };
        if storage::write_atomic(&path, png.get_ref()).is_err() {
            continue;
        }
        files.push(json!({ "index": n, "path": path, "width": w, "height": h, "drawn": drawn }));
    }
    Outcome::ok(json!({ "count": files.len(), "files": files }))
}
