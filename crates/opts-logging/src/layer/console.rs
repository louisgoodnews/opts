//! Terminal console formatting layer with ANSI coloring and verbosity levels.

use std::io::IsTerminal;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Metadata};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

use crate::config::{AnsiMode, ConsoleConfig, ConsoleVerbosity};

/// Custom event formatter implementing [`FormatEvent`] for terminal console output.
#[derive(Debug, Clone)]
pub struct ConsoleFormatter {
    ansi_enabled: bool,
    verbosity: ConsoleVerbosity,
}

impl ConsoleFormatter {
    /// Creates a new `ConsoleFormatter` based on the given [`ConsoleConfig`].
    pub fn new(config: &ConsoleConfig) -> Self {
        let ansi_enabled = match config.ansi {
            AnsiMode::Always => true,
            AnsiMode::Never => false,
            AnsiMode::Auto => std::io::stdout().is_terminal(),
        };

        Self {
            ansi_enabled,
            verbosity: config.verbosity,
        }
    }

    /// Formats the level badge according to ANSI preferences.
    fn format_level(&self, level: &Level) -> String {
        if self.ansi_enabled {
            match *level {
                Level::ERROR => format!("\x1b[1;31m{:5}\x1b[0m", "ERROR"),
                Level::WARN => format!("\x1b[1;33m{:5}\x1b[0m", "WARN"),
                Level::INFO => format!("\x1b[32m{:5}\x1b[0m", "INFO"),
                Level::DEBUG => format!("\x1b[34m{:5}\x1b[0m", "DEBUG"),
                Level::TRACE => format!("\x1b[2;35m{:5}\x1b[0m", "TRACE"),
            }
        } else {
            format!("{:5}", level.as_str())
        }
    }

    /// Formats current time as HH:MM:SS.mmm.
    fn format_time(&self) -> String {
        let now = time::OffsetDateTime::now_utc();
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            now.hour(),
            now.minute(),
            now.second(),
            now.millisecond()
        )
    }
}

impl<S, N> FormatEvent<S, N> for ConsoleFormatter
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
        let time_str = self.format_time();
        let level_str = self.format_level(meta.level());

        // Extract message and custom fields
        let mut visitor = ConsoleVisitor::default();
        event.record(&mut visitor);

        let message = visitor.message.unwrap_or_default();

        match self.verbosity {
            ConsoleVerbosity::Compact => {
                // Minimal: TIME LEVEL message {fields}
                write!(writer, "{time_str} {level_str} {message}")?;
                self.write_fields(&mut writer, &visitor.fields)?;
                writeln!(writer)?;
            }
            ConsoleVerbosity::Standard => {
                // Default: TIME LEVEL target [spans] message {fields}
                let target_str = if self.ansi_enabled {
                    format!("\x1b[2m{}\x1b[0m", meta.target())
                } else {
                    meta.target().to_string()
                };

                write!(writer, "{time_str}  {level_str}  {target_str}  ")?;

                // Print active spans
                if let Some(scope) = ctx.event_scope() {
                    for span in scope {
                        write!(writer, "[{}] ", span.name())?;
                    }
                }

                write!(writer, "{message}")?;
                self.write_fields(&mut writer, &visitor.fields)?;
                writeln!(writer)?;
            }
            ConsoleVerbosity::Verbose => {
                // Verbose: TIME LEVEL target (file:line) [spans] message {fields}
                let target_str = if self.ansi_enabled {
                    format!("\x1b[2m{}\x1b[0m", meta.target())
                } else {
                    meta.target().to_string()
                };

                let loc = match (meta.file(), meta.line()) {
                    (Some(file), Some(line)) => format!(" ({file}:{line})"),
                    _ => String::new(),
                };

                write!(writer, "{time_str}  {level_str}  {target_str}{loc}  ")?;

                if let Some(scope) = ctx.event_scope() {
                    for span in scope {
                        write!(writer, "[{}] ", span.name())?;
                    }
                }

                write!(writer, "{message}")?;
                self.write_fields(&mut writer, &visitor.fields)?;
                writeln!(writer)?;
            }
        }

        Ok(())
    }
}

impl ConsoleFormatter {
    fn write_fields(
        &self,
        writer: &mut Writer<'_>,
        fields: &[(String, String)],
    ) -> std::fmt::Result {
        if !fields.is_empty() {
            write!(writer, " {{")?;
            for (idx, (k, v)) in fields.iter().enumerate() {
                if idx > 0 {
                    write!(writer, ", ")?;
                }
                if self.ansi_enabled {
                    write!(writer, "\x1b[2m{k}\x1b[0m={v}")?;
                } else {
                    write!(writer, "{k}={v}")?;
                }
            }
            write!(writer, "}}")?;
        }
        Ok(())
    }
}

/// Visitor that separates the log event message from additional key-value fields.
#[derive(Default)]
struct ConsoleVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl Visit for ConsoleVisitor {
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
