//! Field visitor of the console formatter: separates the message from the key-value fields.

use tracing::field::{Field, Visit};

/// Visitor that separates the log event message from additional key-value fields.
#[derive(Default)]
pub(super) struct ConsoleVisitor {
    pub(super) message: Option<String>,
    pub(super) fields: Vec<(String, String)>,
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
