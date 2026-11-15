//! Checks of the judgement that need no pixels: diagnostics, bound variables and changed variables.

use super::*;

/// Check 5: diagnostics.
pub(super) fn errors(j: &Judge<'_>, out: &mut Judgement) {
    if !j.outcome.engine_found {
        add(out, "S-NOT-RUN");
        out.check
            .errors
            .push("the object spawned without a script engine".into());
    }
    for d in &j.outcome.diagnostics {
        let missing = d.message == "file not found";
        let text = format!(
            "{}: {}",
            d.file.rsplit(['/', '\\']).next().unwrap_or(""),
            d.message
        );
        if missing {
            add(out, "S-MISSING");
        } else if d.error || d.runtime {
            add(out, "S-ERROR");
        } else {
            continue;
        }
        if out.check.errors.len() < 8 {
            out.check.errors.push(text);
        }
    }
}

/// The bound variables and their values per state.
pub(super) fn bound_vars(j: &Judge<'_>, check: &mut ScriptCheck) {
    let mut names: Vec<String> = Vec::new();
    for m in &j.meshes {
        names.extend(m.visible.iter().map(|v| v.0.to_lowercase()));
        names.extend(m.anim.iter().map(|a| a.0.to_lowercase()));
    }
    names.extend(
        j.sco
            .lights
            .iter()
            .filter(|l| !l.variable.trim().is_empty())
            .map(|l| l.variable.trim().to_lowercase()),
    );
    names.sort();
    names.dedup();
    for n in names.into_iter().take(12) {
        let vals = j.outcome.states.iter().map(|s| var(s, &n)).collect();
        check.bound.insert(n, vals);
    }
}

pub(super) fn changed_vars(j: &Judge<'_>, check: &mut ScriptCheck) {
    let mut names: Vec<&String> = j
        .outcome
        .states
        .iter()
        .flat_map(|s| s.probe.vars.keys())
        .collect();
    names.sort();
    names.dedup();
    for n in names {
        let vals: Vec<f32> = j
            .outcome
            .states
            .iter()
            .map(|s| s.probe.vars.get(n).copied().unwrap_or(0.0))
            .collect();
        if vals.iter().any(|v| (v - vals[0]).abs() > 1e-4)
            && check.changed_vars.len() < MAX_CHANGED_VARS
        {
            check.changed_vars.push(n.clone());
        }
    }
}
