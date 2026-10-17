//! Named timers for the tile-streaming pipeline (M-0089, `docs/src/architecture/streaming-performance.md`).
//!
//! `let _t = stream_trace::span("tile.parse");` times the rest of the scope. Without the cargo
//! feature `stream-trace` the guard is a zero-sized type with an empty destructor and `span` is an
//! empty `#[inline(always)]` function: no clock read, no lock, no code. With the feature every
//! drop adds the elapsed time to a process-wide table keyed by the static name; the host drains
//! the table once per frame ([`drain`]) and writes one line per frame
//! (`OPTS_STREAM_TRACE=<file>` in `opts-game`).
//!
//! Times are **inclusive**: a span nested in another is counted in both. Spans may run on any
//! thread (decode workers record under their own names); the per-frame line sums all threads.

#[cfg(feature = "stream-trace")]
mod on {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Instant;

    /// Summed time, run count and longest run of one span name.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct Stat {
        /// Number of finished spans.
        pub count: u32,
        /// Summed time in nanoseconds.
        pub total_ns: u64,
        /// Longest single run in nanoseconds.
        pub max_ns: u64,
    }

    static TABLE: Mutex<Option<HashMap<&'static str, Stat>>> = Mutex::new(None);

    /// Guard returned by [`span`]; records on drop.
    #[must_use = "the span ends when the guard is dropped"]
    pub struct Span {
        name: &'static str,
        start: Instant,
    }

    /// Starts timing `name` until the guard is dropped.
    #[inline]
    pub fn span(name: &'static str) -> Span {
        Span {
            name,
            start: Instant::now(),
        }
    }

    impl Drop for Span {
        fn drop(&mut self) {
            let ns = u64::try_from(self.start.elapsed().as_nanos()).unwrap_or(u64::MAX);
            if let Ok(mut t) = TABLE.lock() {
                let s = t.get_or_insert_with(HashMap::new).entry(self.name).or_default();
                s.count += 1;
                s.total_ns += ns;
                s.max_ns = s.max_ns.max(ns);
            }
        }
    }

    /// Takes everything recorded since the last call, sorted by name.
    pub fn drain() -> Vec<(&'static str, Stat)> {
        let Ok(mut t) = TABLE.lock() else {
            return Vec::new();
        };
        let mut v: Vec<_> = t.take().unwrap_or_default().into_iter().collect();
        v.sort_by_key(|(n, _)| *n);
        v
    }

    /// Whether the timers are compiled in.
    pub const ENABLED: bool = true;
}

#[cfg(not(feature = "stream-trace"))]
mod off {
    /// Summed time, run count and longest run of one span name (always empty without the feature).
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct Stat {
        /// Number of finished spans.
        pub count: u32,
        /// Summed time in nanoseconds.
        pub total_ns: u64,
        /// Longest single run in nanoseconds.
        pub max_ns: u64,
    }

    /// Zero-sized guard; its (empty) destructor is optimised away.
    #[derive(Debug)]
    pub struct Span;

    impl Drop for Span {
        #[inline(always)]
        fn drop(&mut self) {}
    }

    /// Does nothing (the `stream-trace` feature is off).
    #[inline(always)]
    #[must_use = "the span ends when the guard is dropped"]
    pub fn span(_name: &'static str) -> Span {
        Span
    }

    /// Always empty (the `stream-trace` feature is off).
    #[inline(always)]
    pub fn drain() -> Vec<(&'static str, Stat)> {
        Vec::new()
    }

    /// Whether the timers are compiled in.
    pub const ENABLED: bool = false;
}

#[cfg(feature = "stream-trace")]
pub use on::{drain, span, Span, Stat, ENABLED};
#[cfg(not(feature = "stream-trace"))]
pub use off::{drain, span, Span, Stat, ENABLED};

/// One JSON line for a frame: `{"f":<frame>,"dt":<ms>,"s":{"<name>":[count,total_ms,max_ms]}}`.
#[must_use]
pub fn frame_line(frame: u64, dt_ms: f64, spans: &[(&'static str, Stat)]) -> String {
    let mut s = format!("{{\"f\":{frame},\"dt\":{dt_ms:.3},\"s\":{{");
    for (i, (name, st)) in spans.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "\"{name}\":[{},{:.3},{:.3}]",
            st.count,
            st.total_ns as f64 / 1e6,
            st.max_ns as f64 / 1e6
        ));
    }
    s.push_str("}}");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_line_is_valid_shape() {
        let st = Stat {
            count: 2,
            total_ns: 3_000_000,
            max_ns: 2_000_000,
        };
        assert_eq!(
            frame_line(7, 16.5, &[("tile.parse", st)]),
            r#"{"f":7,"dt":16.500,"s":{"tile.parse":[2,3.000,2.000]}}"#
        );
    }

    #[test]
    fn spans_record_only_when_enabled() {
        let _ = drain();
        {
            let _t = span("unit.test");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let got: Vec<_> = drain().into_iter().filter(|(n, _)| *n == "unit.test").collect();
        if ENABLED {
            assert_eq!(got.len(), 1);
            assert_eq!(got[0].1.count, 1);
            assert!(got[0].1.total_ns >= 1_000_000);
        } else {
            assert!(got.is_empty());
            assert_eq!(std::mem::size_of::<Span>(), 0);
        }
    }
}
