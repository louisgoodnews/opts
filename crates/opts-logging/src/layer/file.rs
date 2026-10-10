//! File formatting layer for subsystem and aggregate log files.

use tracing::field::{Field, Visit};
use tracing::{Event, Metadata};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// Text formatter for file appenders with full ISO 8601 timestamps and structured spans.
#[derive(Debug, Clone, Default)]
pub struct FileTextFormatter;

impl FileTextFormatter {
    /// Formats the current time in RFC 3339 / ISO 8601 with millisecond precision.
    fn format_timestamp() -> String {
        let now = time::OffsetDateTime::now_utc();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            now.year(),
            now.month() as u8,
            now.day(),
            now.hour(),
            now.minute(),
            now.second(),
            now.millisecond()
        )
    }
}

impl<S, N> FormatEvent<S, N> for FileTextFormatter
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let meta: &Metadata<'_> = event.metadata();
        let timestamp = Self::format_timestamp();
        let level = meta.level().as_str();
        let target = meta.target();

        let mut visitor = FileVisitor::default();
        event.record(&mut visitor);

        let message = visitor.message.unwrap_or_default();

        // Format: YYYY-MM-DDTHH:MM:SS.mmmZ  LEVEL  target  [spans]  message  {fields}
        write!(writer, "{timestamp}  {level:5}  {target}  ")?;

        if let Some(scope) = ctx.event_scope() {
            for span in scope {
                write!(writer, "[{}] ", span.name())?;
            }
        }

        write!(writer, "{message}")?;

        if !visitor.fields.is_empty() {
            write!(writer, " {{")?;
            for (i, (k, v)) in visitor.fields.iter().enumerate() {
                if i > 0 {
                    write!(writer, ", ")?;
                }
                write!(writer, "{k}={v}")?;
            }
            write!(writer, "}}")?;
        }

        writeln!(writer)?;
        Ok(())
    }
}

#[derive(Default)]
struct FileVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl Visit for FileVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}").trim_matches('"').to_string());
        } else {
            self.fields
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }
}
