//! Plain-text rendering of a profile for the terminal, the settings page and bug reports.
//! Generated from the serialised form, so a new field appears everywhere without further edits.

use serde_json::Value;

use crate::profile::HardwareProfile;
use crate::recommend::{
    dlss_available, gpu_tier, recommended_look, recommended_texture_filtering,
    recommended_tile_radius, recommended_ui_scale, Availability, GpuTier, MapStats,
};

fn leaf(v: &Value) -> String {
    match v {
        Value::Array(a) => {
            let items: Vec<String> = a.iter().map(leaf).collect();
            if items.is_empty() {
                "none".to_owned()
            } else {
                items.join(" ")
            }
        }
        Value::String(s) => s.clone(),
        Value::Object(m) if m.contains_key("major") => {
            format!(
                "{}.{}",
                m["major"],
                m.get("minor").unwrap_or(&Value::from(0))
            )
        }
        other => other.to_string(),
    }
}

fn walk(prefix: &str, v: &Value, out: &mut Vec<(String, String)>) {
    let Value::Object(m) = v else { return };
    if m.len() == 1 {
        if let Some(k) = m.get("known") {
            out.push((prefix.to_owned(), leaf(k)));
            return;
        }
        if let Some(r) = m.get("unknown") {
            out.push((prefix.to_owned(), format!("unknown ({})", leaf(r))));
            return;
        }
    }
    for (k, v) in m {
        let p = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        walk(&p, v, out);
    }
}

/// `(field path, value text)` for every fact, in profile order.
#[must_use]
pub fn fact_lines(p: &HardwareProfile) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Ok(v) = serde_json::to_value(p) {
        walk("", &v, &mut out);
    }
    out
}

/// The recommendations for this profile with the default [`MapStats`].
#[must_use]
pub fn recommendation_lines(p: &HardwareProfile) -> Vec<(String, String)> {
    let tier = match gpu_tier(p) {
        GpuTier::Unknown => "unknown",
        GpuTier::Low => "low",
        GpuTier::Mid => "mid",
        GpuTier::High => "high",
    };
    let dlss = match dlss_available(p) {
        Availability::Yes => "yes".to_owned(),
        Availability::No(r) => format!("no ({})", r.key()),
        Availability::Unknown => "unknown".to_owned(),
    };
    vec![
        ("gpu_tier".into(), tier.into()),
        (
            "tile_radius".into(),
            recommended_tile_radius(p, &MapStats::default()).to_string(),
        ),
        (
            "texture_filtering".into(),
            recommended_texture_filtering(p).into(),
        ),
        ("look".into(), recommended_look(p).into()),
        ("ui_scale".into(), recommended_ui_scale(p).to_string()),
        ("dlss".into(), dlss),
    ]
}

/// The whole report as aligned text.
#[must_use]
pub fn render_text(p: &HardwareProfile) -> String {
    let facts = fact_lines(p);
    let recs = recommendation_lines(p);
    let width = facts
        .iter()
        .chain(&recs)
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(0);
    let mut s = format!(
        "hardware profile (schema {}, probe {} ms)\n",
        p.schema, p.probe_ms
    );
    for (k, v) in &facts {
        s.push_str(&format!("  {k:<width$}  {v}\n"));
    }
    s.push_str("recommended defaults\n");
    for (k, v) in &recs {
        s.push_str(&format!("  {k:<width$}  {v}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact::Fact;

    #[test]
    fn lines_cover_every_field_and_show_reasons() {
        let mut p = HardwareProfile::default();
        p.cpu.logical_cores = Fact::Known(8);
        p.cpu.features = Fact::Known(vec!["avx2".into(), "fma".into()]);
        let lines = fact_lines(&p);
        assert_eq!(lines.len(), crate::reference::FIELDS.len());
        let get = |k: &str| lines.iter().find(|(p, _)| p == k).map(|(_, v)| v.as_str());
        assert_eq!(get("cpu.logical_cores"), Some("8"));
        assert_eq!(get("cpu.features"), Some("avx2 fma"));
        assert_eq!(get("cpu.arch"), Some("unknown (not_probed)"));
        assert!(render_text(&p).contains("tile_radius"));
    }
}
