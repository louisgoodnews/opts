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
use opts_core::formats::script_texture::ScriptTextureCanvas;

/// Most canvases written by one call.
const MAX_FILES: usize = 64;

/// Registers the method.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.script_textures", script_textures);
}

/// `vehicle.script_textures {prefix?}`: one PNG per canvas, named `<prefix>-<unit>-<slot>.png`
/// (default prefix `canvas`): the canvases of each loaded unit, what the scripts drew on every slot
/// of the model's `[scripttexture]` list, shown on a mesh or not.
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
    // The canvases of every loaded unit (the CPU side: what the scripts drew, also on slots no
    // mesh shows), once per set.
    let sets: Vec<
        std::sync::Arc<
            std::sync::RwLock<Vec<opts_core::formats::script_texture::CpuScriptTexture>>,
        >,
    > = {
        let mut q = world.query::<&VehicleScriptTexture>();
        let mut seen: Vec<std::sync::Arc<std::sync::RwLock<Vec<_>>>> = Vec::new();
        for t in q.iter(world) {
            if !seen.iter().any(|c| std::sync::Arc::ptr_eq(c, &t.canvases)) {
                seen.push(t.canvases.clone());
            }
        }
        seen
    };
    let mut files = Vec::new();
    'sets: for (unit, set) in sets.iter().enumerate() {
        let Ok(canvases) = set.read() else {
            continue;
        };
        for (slot, canvas) in canvases.iter().enumerate() {
            if files.len() >= MAX_FILES {
                break 'sets;
            }
            let (w, h) = (canvas.width(), canvas.height());
            let data = canvas.raw_rgba();
            let drawn = data.chunks(4).any(|p| p != [0, 0, 0, 0]);
            let Some(buffer) = image::RgbaImage::from_raw(w, h, data.to_vec()) else {
                continue;
            };
            let mut png = std::io::Cursor::new(Vec::new());
            if buffer.write_to(&mut png, image::ImageFormat::Png).is_err() {
                continue;
            }
            let path = match storage::resolve_name(&root, &format!("{prefix}-{unit}-{slot}.png")) {
                Ok(p) => p,
                Err(e) => return Outcome::err(RpcError::invalid_params(e.to_string())),
            };
            if storage::write_atomic(&path, png.get_ref()).is_err() {
                continue;
            }
            files.push(json!({ "unit": unit, "slot": slot, "path": path, "width": w, "height": h, "drawn": drawn }));
        }
    }
    // What reached the images of the meshes (the GPU side): per slot, whether any pixel has alpha.
    let mut q = world.query::<&VehicleScriptTexture>();
    let targets: Vec<(usize, Option<[u8; 3]>, Handle<Image>)> = q
        .iter(world)
        .map(|t| (t.index, t.tint, t.image.clone()))
        .collect();
    let images = world.resource::<Assets<Image>>();
    let on_meshes: Vec<Value> = targets
        .iter()
        .map(|(index, tint, image)| {
            let lit = images
                .get(image)
                .and_then(|i| i.data.as_ref())
                .map(|d| d.chunks(4).filter(|p| p[3] != 0).count());
            json!({ "slot": index, "tint": tint, "pixels_with_alpha": lit })
        })
        .collect();
    Outcome::ok(json!({ "count": files.len(), "files": files, "on_meshes": on_meshes }))
}
