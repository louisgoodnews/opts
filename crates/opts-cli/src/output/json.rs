//! Machine-readable output.

use serde_json::{json, Value};

use crate::protocol::RpcError;

/// `{"result": …}` as pretty JSON.
#[must_use]
pub fn format_result(result: &Value) -> String {
    serde_json::to_string_pretty(&json!({ "result": result })).unwrap_or_default()
}

/// `{"error": {code, message, data?}}` as pretty JSON.
#[must_use]
pub fn format_error(error: &RpcError) -> String {
    serde_json::to_string_pretty(&json!({ "error": error })).unwrap_or_default()
}
