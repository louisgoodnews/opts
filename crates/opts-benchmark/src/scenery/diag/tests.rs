//! Unit tests for diag.

use super::*;

fn ev(level: &str, template: &str) -> EventRecord {
    EventRecord {
        level: level.into(),
        target: "opts_bevy::model".into(),
        template: template.into(),
        detail: String::new(),
    }
}

#[test]
fn events_map_to_categories() {
    let events = [
        ev("ERROR", "texture not found: a.dds (searched 3 folders)"),
        ev("WARN", "mesh vertex positions are scrambled; skipped"),
        ev("ERROR", "something else broke"),
        ev("WARN", "harmless"),
        ev("ERROR", "map has parking bays but no parklist_p.txt"),
    ];
    let cats = load_categories(&events, &BTreeMap::new());
    assert_eq!(
        cats,
        vec!["F-TEX-MISSING", "F-MESH-PROTECTED", "F-LOAD-ERROR"]
    );
}

#[test]
fn capture_layer_records_opts_warnings_only() {
    use tracing_subscriber::layer::SubscriberExt;
    let sink: EventSink = Arc::default();
    let sub = tracing_subscriber::registry().with(CaptureLayer::new(sink.clone(), "/omsi"));
    tracing::subscriber::with_default(sub, || {
        tracing::warn!(target: "opts_bevy::model", path = "/omsi/x.o3d", "bad mesh /omsi/x.o3d");
        tracing::info!(target: "opts_bevy::model", "ignored");
        tracing::error!(target: "wgpu", "ignored too");
    });
    let got = sink.lock().unwrap().clone();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].template, "bad mesh <root>/x.o3d");
    assert_eq!(got[0].detail, "path=<root>/x.o3d");
}
