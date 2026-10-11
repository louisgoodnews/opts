//! Per-object diagnostics: a tracing layer that captures `WARN`/`ERROR` events of the OPTS
//! crates, `WorldStats` deltas, and the load-stage categories derived from both.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

use super::EventRecord;

/// Shared buffer of captured events.
pub type EventSink = Arc<Mutex<Vec<EventRecord>>>;

/// Captures `WARN` and `ERROR` events whose target starts with `opts`.
#[derive(Debug, Clone)]
pub struct CaptureLayer {
    sink: EventSink,
    root: String,
}

impl CaptureLayer {
    /// A layer writing into `sink`; occurrences of `root` in messages become `<root>`.
    #[must_use]
    pub fn new(sink: EventSink, root: &str) -> Self {
        Self {
            sink,
            root: root.to_string(),
        }
    }
}

#[derive(Default)]
struct Fields {
    message: String,
    detail: String,
}

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else if field.name() != "at" && self.detail.len() < 300 {
            let _ = write!(self.detail, "{}={value:?} ", field.name());
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else if field.name() != "at" && self.detail.len() < 300 {
            let _ = write!(self.detail, "{}={value} ", field.name());
        }
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let level = *meta.level();
        if level > Level::WARN || !meta.target().starts_with("opts") {
            return;
        }
        let mut f = Fields::default();
        event.record(&mut f);
        let clean = |s: &str| {
            if self.root.is_empty() {
                s.to_string()
            } else {
                s.replace(&self.root, "<root>")
            }
        };
        let record = EventRecord {
            level: level.to_string(),
            target: meta.target().to_string(),
            template: clean(&f.message),
            detail: clean(f.detail.trim_end()),
        };
        if let Ok(mut v) = self.sink.lock() {
            v.push(record);
        }
    }
}

/// Events seen while loading the empty map (anchor only): they say nothing about an object.
#[derive(Debug, Clone, Default)]
pub struct Baseline(HashSet<(String, String)>);

impl Baseline {
    /// Remembers `events` as baseline.
    #[must_use]
    pub fn from_events(events: &[EventRecord]) -> Self {
        Self(
            events
                .iter()
                .map(|e| (e.target.clone(), e.template.clone()))
                .collect(),
        )
    }

    /// Drops baseline events.
    #[must_use]
    pub fn filter(&self, events: Vec<EventRecord>) -> Vec<EventRecord> {
        events
            .into_iter()
            .filter(|e| !self.0.contains(&(e.target.clone(), e.template.clone())))
            .collect()
    }
}

/// Load-stage categories from the captured events and the counter deltas.
#[must_use]
pub fn load_categories(events: &[EventRecord], stats: &BTreeMap<String, i64>) -> Vec<String> {
    let mut cats: Vec<String> = Vec::new();
    let mut add = |c: &str| {
        if !cats.iter().any(|x| x == c) {
            cats.push(c.to_string());
        }
    };
    if stats.get("objects_protected").is_some_and(|n| *n > 0) {
        add("F-MESH-PROTECTED");
    }
    for e in events {
        let t = e.template.to_lowercase();
        let category = if t.starts_with("texture not found")
            || t.starts_with("texture decode failed")
            || t.starts_with("texture read failed")
        {
            Some("F-TEX-MISSING")
        } else if t.starts_with("o3d not found") {
            Some("F-MESH-MISSING")
        } else if t.starts_with("mesh parse failed") || t.starts_with("o3d read failed") {
            Some("F-MESH-PARSE")
        } else if t.contains("scrambled; skipped") || t.contains("extent is implausible") {
            Some("F-MESH-PROTECTED")
        } else if t.starts_with("sco parse failed") {
            Some("F-SCO-PARSE")
        } else if e.level == "ERROR" {
            Some("F-LOAD-ERROR")
        } else {
            None
        };
        if let Some(c) = category {
            add(c);
        }
    }
    cats
}

#[cfg(test)]
mod tests {
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
}
