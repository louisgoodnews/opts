//! In-memory ring buffer of recent log events, queried by the control protocol (`log.tail`).

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::{Mutex, OnceLock};

use time::macros::format_description;
use time::OffsetDateTime;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Events kept; older ones are dropped first.
pub const RING_CAPACITY: usize = 2000;

/// One captured event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    /// UTC time of day (`HH:MM:SS.mmm`).
    pub ts: String,
    /// `TRACE`, `DEBUG`, `INFO`, `WARN` or `ERROR`.
    pub level: &'static str,
    /// Module path of the event.
    pub target: String,
    /// The message followed by `key=value` fields.
    pub message: String,
}

/// Which records [`recent`] returns.
#[derive(Debug, Clone, Default)]
pub struct LogQuery {
    /// Keep records whose target starts with this prefix.
    pub target: Option<String>,
    /// Keep records at this level or more severe.
    pub min_level: Option<Level>,
    /// Keep records whose message contains this text.
    pub contains: Option<String>,
}

static RING: OnceLock<Mutex<VecDeque<LogRecord>>> = OnceLock::new();

fn ring() -> &'static Mutex<VecDeque<LogRecord>> {
    RING.get_or_init(|| Mutex::new(VecDeque::with_capacity(RING_CAPACITY)))
}

fn severity(level: &str) -> u8 {
    match level {
        "ERROR" => 4,
        "WARN" => 3,
        "INFO" => 2,
        "DEBUG" => 1,
        _ => 0,
    }
}

fn level_name(level: &Level) -> &'static str {
    match *level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARN",
        Level::INFO => "INFO",
        Level::DEBUG => "DEBUG",
        Level::TRACE => "TRACE",
    }
}

/// Records pushed by [`RingLayer`], newest last, filtered by `query`, at most `n` of them.
#[must_use]
pub fn recent(n: usize, query: &LogQuery) -> Vec<LogRecord> {
    let Ok(guard) = ring().lock() else {
        return Vec::new();
    };
    let min = query
        .min_level
        .as_ref()
        .map_or(0, |l| severity(level_name(l)));
    let mut out: Vec<LogRecord> = guard
        .iter()
        .rev()
        .filter(|r| severity(r.level) >= min)
        .filter(|r| {
            query
                .target
                .as_ref()
                .is_none_or(|t| r.target.starts_with(t.as_str()))
        })
        .filter(|r| {
            query
                .contains
                .as_ref()
                .is_none_or(|c| r.message.contains(c.as_str()))
        })
        .take(n)
        .cloned()
        .collect();
    out.reverse();
    out
}

/// Tracing layer feeding the ring buffer.
#[derive(Debug, Default, Clone, Copy)]
pub struct RingLayer;

struct MessageVisitor {
    message: String,
    fields: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }
}

impl<S: Subscriber> Layer<S> for RingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut v = MessageVisitor {
            message: String::new(),
            fields: String::new(),
        };
        event.record(&mut v);
        let now = OffsetDateTime::now_utc();
        let ts = now
            .format(format_description!(
                "[hour]:[minute]:[second].[subsecond digits:3]"
            ))
            .unwrap_or_default();
        let record = LogRecord {
            ts,
            level: level_name(event.metadata().level()),
            target: event.metadata().target().to_string(),
            message: format!("{}{}", v.message, v.fields),
        };
        if let Ok(mut guard) = ring().lock() {
            if guard.len() == RING_CAPACITY {
                guard.pop_front();
            }
            guard.push_back(record);
        }
    }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;

    #[test]
    fn captures_and_filters_events() {
        let subscriber = tracing_subscriber::registry().with(RingLayer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "ring_test::a", "hello world");
            tracing::warn!(target: "ring_test::b", code = 7, "careful");
            tracing::debug!(target: "ring_test::a", "noise");
        });
        let all = recent(
            50,
            &LogQuery {
                target: Some("ring_test".into()),
                ..LogQuery::default()
            },
        );
        assert_eq!(all.len(), 3);
        let warn = recent(
            50,
            &LogQuery {
                target: Some("ring_test".into()),
                min_level: Some(Level::WARN),
                ..LogQuery::default()
            },
        );
        assert_eq!(warn.len(), 1);
        assert_eq!(warn[0].message, "careful code=7");
        let found = recent(
            50,
            &LogQuery {
                contains: Some("hello".into()),
                target: Some("ring_test".into()),
                ..LogQuery::default()
            },
        );
        assert_eq!(found[0].level, "INFO");
        let last = recent(
            1,
            &LogQuery {
                target: Some("ring_test".into()),
                ..LogQuery::default()
            },
        );
        assert_eq!(last[0].message, "noise");
    }
}
