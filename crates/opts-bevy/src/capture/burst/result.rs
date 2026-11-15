//! The result of a finished burst: the manifest and the reply.

use serde_json::{json, Value};

use opts_cli::protocol::RpcError;

use super::super::error::CaptureError;
use super::super::storage;
use super::state::{Burst, Frame};

impl Burst {
    pub(super) fn result(&mut self) -> Result<Value, RpcError> {
        let ok: Vec<&Frame> = self.frames.iter().filter(|f| f.path.is_some()).collect();
        if ok.is_empty() {
            return Err(CaptureError::Failed(format!(
                "no frame was captured: {}",
                self.last_error.as_deref().unwrap_or("unknown cause")
            ))
            .into());
        }
        let frames: Vec<Value> = ok
            .iter()
            .map(|f| {
                json!({
                    "index": f.index,
                    "path": f.path.as_ref().map(|p| p.display().to_string()),
                    "sim_time": f.sim_time,
                    "frame": f.frame,
                    "wall_ms": f.wall_ms,
                })
            })
            .collect();
        let paths: Vec<Value> = frames.iter().map(|f| f["path"].clone()).collect();
        let mut manifest = self.base.take().unwrap_or_default();
        manifest.insert("id".into(), json!(self.id));
        manifest.insert("count".into(), json!(self.count));
        manifest.insert("interval_ms".into(), json!((self.interval * 1000.0) as u64));
        manifest.insert("dropped".into(), json!(self.dropped));
        manifest.insert("frames".into(), json!(frames));
        let manifest_path = self.dir.join("manifest.json");
        let text = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| CaptureError::Failed(format!("manifest serialisation failed: {e}")))?;
        storage::write_atomic(&manifest_path, &text)?;
        Ok(json!({
            "dir": self.dir.display().to_string(),
            "paths": paths,
            "frames": frames,
            "dropped": self.dropped,
            "manifest": manifest_path.display().to_string(),
        }))
    }
}
