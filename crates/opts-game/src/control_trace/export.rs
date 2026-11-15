//! `trace.export`: writes matching in-memory records as JSON Lines.

use super::*;

/// `trace.export {path, file?, kind?, tile?, issues_only?}`: writes the matching in-memory
/// records as JSON Lines (the run file next to the logs holds everything).
pub(super) fn export(world: &mut World, params: &Value) -> Outcome {
    if !st::enabled() {
        return Outcome::err(not_on());
    }
    let raw = match str_param(params, "path") {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let path = match world
        .resource::<PathPolicy>()
        .check_write(&PathBuf::from(raw))
    {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let tile = match tile_param(params) {
        Ok(t) => t,
        Err(e) => return Outcome::err(e),
    };
    let mut q = match run_query(
        params,
        TraceQuery {
            tile,
            ..TraceQuery::default()
        },
    ) {
        Ok(q) => q,
        Err(e) => return Outcome::err(e),
    };
    q.limit = opt_u64(params, "limit")
        .ok()
        .flatten()
        .map_or(usize::MAX / 2, |n| n as usize);
    let lines = st::export_lines(&q);
    let mut text = lines.join("\n");
    text.push('\n');
    match std::fs::write(&path, text) {
        Ok(()) => Outcome::ok(json!({"path": path.display().to_string(), "records": lines.len()})),
        Err(e) => Outcome::err(RpcError::new(code::FAILED, format!("write failed: {e}"))),
    }
}
