//! The JSON views of the map profiles of the control protocol (M-0130): the names of the sources,
//! the advice, a profile as the protocol shows it and the description of what the plan's map has.

use serde_json::{json, Value};

use crate::map_profiles::advice::{Advice, AdviceId, Severity};
use crate::map_profiles::service::ProfileView;
use crate::map_profiles::store::ProfileSource;
use crate::map_profiles::MapProfile;

pub(super) fn source_name(s: ProfileSource) -> &'static str {
    match s {
        ProfileSource::Chrono => "chrono",
        ProfileSource::Map => "map",
        ProfileSource::Global => "global",
    }
}

fn advice_id(id: AdviceId) -> &'static str {
    match id {
        AdviceId::GeometryAboveTypical => "geometry-above-typical",
        AdviceId::GeometryHigh => "geometry-high",
        AdviceId::LightMap => "light-map",
        AdviceId::ManySplines => "many-splines",
        AdviceId::HighAiLoad => "high-ai-load",
        AdviceId::RanSlowly => "ran-slowly",
        AdviceId::RanSmoothly => "ran-smoothly",
        AdviceId::NotPlayedYet => "not-played-yet",
    }
}

fn advice_json(a: &Advice) -> Value {
    json!({
        "id": advice_id(a.id),
        "severity": match a.severity { Severity::Info => "info", Severity::Warning => "warning" },
        "preset": a.preset,
        "suggested": a.suggested.iter().map(|(k, v)| json!({ "key": k, "value": v })).collect::<Vec<_>>(),
    })
}

fn profile_json(chrono: Option<&str>, p: &MapProfile) -> Value {
    json!({
        "chrono": chrono,
        "name": p.name,
        "created": p.created,
        "based_on": p.based_on,
        "settings": p.settings,
    })
}

pub(super) fn describe(v: &ProfileView) -> Value {
    json!({
        "map": v.key.map_id,
        "chrono": v.key.chrono_id,
        "chrono_entries": v.chrono_entries,
        "resolved": {
            "source": source_name(v.resolved.source),
            "profile": v.resolved.profile.as_ref().map(|p| p.name.clone()),
            "dropped": v.resolved.dropped.iter().map(|d| json!({ "key": d.key, "reason": d.reason })).collect::<Vec<_>>(),
        },
        "picked": v.picked,
        "saved": v.saved.iter().map(|(c, p)| profile_json(Some(c), p)).collect::<Vec<_>>(),
        "presets": v.presets.iter().map(|p| profile_json(None, p)).collect::<Vec<_>>(),
        "load": {
            "tiles": v.load.tile_count,
            "objects": v.load.object_count,
            "splines": v.load.spline_count,
            "ai_pool_vehicles": v.load.ai_pool_vehicles,
            "area_km2": v.load.map_size_km2,
            "estimated": v.load.estimated,
            "counting": v.counting,
            "avg_frame_time_ms": v.load.avg_frame_time_ms,
            "p95_frame_time_ms": v.load.p95_frame_time_ms,
        },
        "advice": v.advice.iter().map(advice_json).collect::<Vec<_>>(),
    })
}
