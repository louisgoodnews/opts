//! Names and the JSON description of a setvar entry.


use super::*;


pub(super) fn kind_name(k: SetvarKind) -> &'static str {
    match k {
        SetvarKind::Switch => "switch",
        SetvarKind::Choice => "choice",
        SetvarKind::Number => "number",
        SetvarKind::Fraction => "fraction",
    }
}

pub(super) fn respawn_name(r: Respawn) -> &'static str {
    match r {
        Respawn::Required => "required",
        Respawn::Live => "live",
        Respawn::Unknown => "unknown",
    }
}

/// The repaint whose values are the defaults: the chosen one, else the vehicle's default.
pub(super) fn repaint_of(src: &Source) -> &str {
    if src.repaint.is_empty() {
        &src.repaints.default_name
    } else {
        &src.repaint
    }
}

pub(super) fn value_of(src: &Source, e: &SetvarEntry) -> f64 {
    src.custom
        .get(&e.name.to_lowercase())
        .map_or_else(|| e.default_for(repaint_of(src)), |v| f64::from(*v))
}

pub(super) fn describe(src: &Source, e: &SetvarEntry) -> Value {
    json!({
        "name": e.name,
        "label": e.label,
        "description": e.description,
        "kind": kind_name(e.kind),
        "min": e.min,
        "max": e.max,
        "values": e.values,
        "value": value_of(src, e),
        "default": e.default_for(repaint_of(src)),
        "group": e.group,
        "declared_in": e.declared_in,
        "evidence": {
            "repaints": e.evidence.observed,
            "declared": e.evidence.declared,
            "documented": e.evidence.documented,
        },
        "range_unverified": e.range_unverified,
        "respawn": respawn_name(e.respawn),
    })
}
