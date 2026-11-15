//! Writing a captured frame: the image file and, after it, the metadata sidecar.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde_json::{json, Map, Value};

use super::super::error::CaptureError;
use super::super::storage;
use super::options::Options;
use super::frame::encode;

/// The sidecar path of a capture file: its name with `.json` appended.
#[must_use]
pub fn sidecar_path(file: &Path) -> PathBuf {
    let mut s = file.as_os_str().to_owned();
    s.push(".json");
    PathBuf::from(s)
}

/// Encodes `image`, writes it to `path` and (with `meta`) the sidecar; returns the reply.
///
/// Runs on a worker thread. The sidecar is written after the image, so its presence means the
/// image is complete.
///
/// # Errors
/// `Failed` on encoding or I/O errors.
pub fn finish(
    image: Image,
    path: &Path,
    options: &Options,
    meta: Option<Map<String, Value>>,
) -> Result<Value, CaptureError> {
    let (width, height) = (image.width(), image.height());
    let bytes = encode(image, options.format, options.quality)?;
    storage::write_atomic(path, &bytes)?;
    let mut reply = Map::new();
    reply.insert("path".into(), json!(path.display().to_string()));
    reply.insert("width".into(), json!(width));
    reply.insert("height".into(), json!(height));
    reply.insert("format".into(), json!(options.format.name()));
    reply.insert("bytes".into(), json!(bytes.len()));
    if let Some(mut meta) = meta {
        meta.insert("width".into(), json!(width));
        meta.insert("height".into(), json!(height));
        let side = sidecar_path(path);
        let text = serde_json::to_vec_pretty(&meta)
            .map_err(|e| CaptureError::Failed(format!("metadata serialisation failed: {e}")))?;
        storage::write_atomic(&side, &text)?;
        for key in ["sim_time", "frame"] {
            if let Some(v) = meta.get(key) {
                reply.insert(key.into(), v.clone());
            }
        }
        reply.insert("metadata_path".into(), json!(side.display().to_string()));
        reply.insert("metadata".into(), Value::Object(meta));
    }
    Ok(Value::Object(reply))
}
