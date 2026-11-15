//! `trace.set`: switches the scene trace mode, filters and overlay.

use super::*;

/// `trace.set {mode?, tiles?, kinds?, file?, radius?, center?, size_cap_mb?, overlay?, reset?}`.
pub(super) fn set(world: &mut World, params: &Value) -> Outcome {
    let mode = match opt_str(params, "mode") {
        Ok(Some(m)) => match Mode::parse(m) {
            Some(m) => Some(m),
            None => {
                return Outcome::err(RpcError::invalid_params(
                    "`mode` must be off, summary or full",
                ))
            }
        },
        Ok(None) => None,
        Err(e) => return Outcome::err(e),
    };
    let filters = match parse_filters(world, params, st::filters()) {
        Ok(f) => f,
        Err(e) => return Outcome::err(e),
    };
    if let Some(mode) = mode {
        st::set_mode(mode);
    }
    if st::enabled() {
        st::set_filters(filters);
        if let Ok(Some(mb)) = opt_u64(params, "size_cap_mb") {
            st::set_size_cap(mb.max(1) * 1024 * 1024);
        }
    }
    if let Ok(Some(on)) = opt_bool(params, "overlay") {
        if let Some(mut o) = world.get_resource_mut::<TraceOverlay>() {
            o.enabled = on;
        }
    }
    let mut out = status_json();
    out["overlay"] = json!(world
        .get_resource::<TraceOverlay>()
        .is_some_and(|o| o.enabled));
    Outcome::ok(out)
}
