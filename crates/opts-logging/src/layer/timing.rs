//! Per-system frame timing from tracing spans (`OPTS_SYSTEM_TIMING=<seconds>`).
//!
//! Bevy emits one `system` span per system run when it is built with its `trace` feature (the
//! `profile` feature of `opts-game`). This layer sums the busy time of every span name over a
//! window and logs the heaviest ones, so a slow scene can be profiled without an external tool.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tracing::span::{Attributes, Id};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

#[derive(Default)]
struct Totals {
    window_start: Option<Instant>,
    /// Name → (summed time, runs, longest run).
    by_name: HashMap<String, (Duration, u32, Duration)>,
}

/// Span-timing layer; reports every `window` of wall-clock time.
pub struct SystemTimingLayer {
    window: Duration,
    top: usize,
    totals: Mutex<Totals>,
}

struct Entered(Instant);
struct Named(String);

impl SystemTimingLayer {
    /// Reports the `top` heaviest spans every `window`.
    #[must_use]
    pub fn new(window: Duration, top: usize) -> Self {
        Self {
            window,
            top,
            totals: Mutex::new(Totals::default()),
        }
    }

    /// The layer when `OPTS_SYSTEM_TIMING` holds a window in seconds.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        let secs: f32 = std::env::var("OPTS_SYSTEM_TIMING").ok()?.parse().ok()?;
        (secs > 0.0).then(|| Self::new(Duration::from_secs_f32(secs), 40))
    }
}

struct NameVisitor(Option<String>);

impl tracing::field::Visit for NameVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "name" {
            self.0 = Some(format!("{value:?}").trim_matches('"').to_owned());
        }
    }
}

impl<S> Layer<S> for SystemTimingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let mut v = NameVisitor(None);
        attrs.record(&mut v);
        let name = match v.0 {
            Some(n) => format!("{}:{n}", attrs.metadata().name()),
            None => attrs.metadata().name().to_owned(),
        };
        span.extensions_mut().insert(Named(name));
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().replace(Entered(Instant::now()));
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(id) else { return };
        let (started, name) = {
            let ext = span.extensions();
            let (Some(e), Some(n)) = (ext.get::<Entered>(), ext.get::<Named>()) else {
                return;
            };
            (e.0, n.0.clone())
        };
        let spent = started.elapsed();
        let Ok(mut t) = self.totals.lock() else {
            return;
        };
        let now = Instant::now();
        let start = *t.window_start.get_or_insert(now);
        let entry = t.by_name.entry(name).or_default();
        entry.0 += spent;
        entry.1 += 1;
        entry.2 = entry.2.max(spent);
        if now.duration_since(start) >= self.window {
            let mut rows: Vec<_> = t.by_name.drain().collect();
            // The heaviest by total time, then the worst single runs (frame spikes) not yet listed.
            rows.sort_by_key(|r| std::cmp::Reverse(r.1 .0));
            let mut spikes: Vec<_> = rows.iter().skip(self.top).cloned().collect();
            spikes.sort_by_key(|r| std::cmp::Reverse(r.1 .2));
            rows.truncate(self.top);
            rows.extend(spikes.into_iter().take(self.top));
            t.window_start = Some(now);
            drop(t);
            let secs = now.duration_since(start).as_secs_f32();
            let mut text =
                format!("system timing over {secs:.1} s (busy time summed over threads):");
            for (name, (sum, runs, max)) in rows {
                text.push_str(&format!(
                    "\n  {:>8.1} ms  {runs:>6} runs  max {:>7.2} ms  {name}",
                    sum.as_secs_f32() * 1000.0,
                    max.as_secs_f32() * 1000.0
                ));
            }
            // Printed directly: the layer sits inside the subscriber, an event would recurse.
            eprintln!("{text}");
        }
    }
}
