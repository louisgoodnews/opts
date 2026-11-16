//! Listing the saves (`session.saves`): each save with its checks, the sections, and the restore
//! and running state.

use bevy::prelude::*;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::RpcError;
use serde_json::{json, Value};

use crate::save::ops::{self};
use crate::save::registry::{preflight, CheckEnv, SaveRegistry};

use super::save_error;
use super::save_load::restore_status;

pub(super) fn saves(world: &mut World, _: &Value) -> Outcome {
    let result = (|| {
        let store = ops::store(world).map_err(|e| save_error(&e))?;
        let registry = world.resource::<SaveRegistry>().clone();
        let root = world
            .get_resource::<crate::in_game::omsi_world::WorldLoadJob>()
            .map(|j| j.omsi_root().to_path_buf())
            .or_else(crate::omsi_root::known_omsi_root);
        let list: Vec<Value> = store
            .list()
            .iter()
            .map(|entry| match &entry.state {
                Ok(l) => {
                    let f = &l.file;
                    let issues = preflight(
                        registry.all(),
                        f,
                        &CheckEnv {
                            root: root.as_deref(),
                            fingerprint: &f.fingerprint,
                        },
                    );
                    json!({
                        "id": entry.id,
                        "name": f.meta.name,
                        "map": f.meta.map,
                        "vehicles": f.meta.vehicles,
                        "play_time_s": f.meta.play_time_s,
                        "created_unix": f.meta.created_unix,
                        "saved_unix": f.meta.saved_unix,
                        "format_version": f.format_version,
                        "opts_version": f.opts_version,
                        "sections": f.sections.iter().map(|(k, v)| json!({"id": k, "version": v.version})).collect::<Vec<_>>(),
                        "unknown_sections": f.sections.keys().filter(|k| registry.get(k).is_none()).collect::<Vec<_>>(),
                        "from_backup": l.from_backup,
                        "issues": issues.iter().map(|i| json!({
                            "kind": format!("{:?}", i.kind),
                            "subject": i.subject,
                            "blocking": i.blocking,
                        })).collect::<Vec<_>>(),
                    })
                }
                Err(e) => json!({
                    "id": entry.id,
                    "error": { "kind": e.kind(), "message": e.to_string() },
                }),
            })
            .collect();
        Ok::<_, RpcError>(json!({
            "dir": store.dir().display().to_string(),
            "saves": list,
            "sections": registry
                .all()
                .iter()
                .map(|s| json!({ "id": s.id, "version": s.version, "about": s.about }))
                .collect::<Vec<_>>(),
            "restore": restore_status(world),
            "running": ops::session_running(world),
        }))
    })();
    result.into()
}
