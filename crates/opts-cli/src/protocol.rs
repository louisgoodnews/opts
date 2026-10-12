//! JSON-RPC 2.0 types of the OPTS control protocol (`docs/src/architecture/control-protocol.md`).
//!
//! One message per line. The same types are used by the client in this crate and by the server in
//! `opts-bevy::control`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Version reported by the `version` method; additive changes bump the minor part.
pub const PROTOCOL_VERSION: &str = "1.1.0";
/// Port the control server uses when it is started without an explicit one.
pub const DEFAULT_PORT: u16 = 44201;
/// Longest accepted line in bytes.
pub const MAX_LINE_BYTES: usize = 1 << 20;
/// Most commands one `batch` may carry.
pub const MAX_BATCH: usize = 100;

/// Error codes (JSON-RPC reserved range and application range).
pub mod code {
    /// The line is not valid JSON.
    pub const PARSE_ERROR: i64 = -32700;
    /// Not a JSON-RPC request object.
    pub const INVALID_REQUEST: i64 = -32600;
    /// Unknown method.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// Missing, mistyped or out-of-range parameter.
    pub const INVALID_PARAMS: i64 = -32602;
    /// Unexpected handler failure.
    pub const INTERNAL_ERROR: i64 = -32603;
    /// The app is not in a state in which the command makes sense.
    pub const NOT_READY: i64 = 1001;
    /// Path outside the allowed roots.
    pub const FORBIDDEN_PATH: i64 = 1002;
    /// Token missing or wrong.
    pub const UNAUTHORIZED: i64 = 1003;
    /// Entity, asset or file does not exist.
    pub const NOT_FOUND: i64 = 1004;
    /// The app did not answer in time.
    pub const TIMEOUT: i64 = 1005;
    /// Rate limit exceeded or queue full.
    pub const BUSY: i64 = 1006;
    /// Specified but not implemented in this build.
    pub const UNSUPPORTED: i64 = 1007;
    /// The operation ran and failed.
    pub const FAILED: i64 = 1008;
    /// The input layer refused the command: a dialog or another gate is blocking it (the same
    /// gate that would swallow a key press).
    pub const GATED: i64 = 1009;

    /// Every code with its symbolic name and whether retrying later can succeed.
    pub const ALL: &[(i64, &str, bool, &str)] = &[
        (PARSE_ERROR, "PARSE_ERROR", false, "the line is not valid JSON"),
        (INVALID_REQUEST, "INVALID_REQUEST", false, "not a JSON-RPC request object"),
        (METHOD_NOT_FOUND, "METHOD_NOT_FOUND", false, "unknown method (see `commands`)"),
        (INVALID_PARAMS, "INVALID_PARAMS", false, "missing, mistyped or out-of-range parameter"),
        (INTERNAL_ERROR, "INTERNAL_ERROR", false, "handler panicked or failed unexpectedly"),
        (NOT_READY, "NOT_READY", true, "the app is not in a state in which the command makes sense (no session, wrong screen, element hidden or disabled)"),
        (FORBIDDEN_PATH, "FORBIDDEN_PATH", false, "path outside the allowed read/write roots"),
        (UNAUTHORIZED, "UNAUTHORIZED", false, "token missing or wrong"),
        (NOT_FOUND, "NOT_FOUND", false, "entity, element, asset or file does not exist"),
        (TIMEOUT, "TIMEOUT", true, "the app (or a `ui.wait_for` condition) did not answer in time"),
        (BUSY, "BUSY", true, "rate limit exceeded or command queue full"),
        (UNSUPPORTED, "UNSUPPORTED", false, "specified but not implemented in this build"),
        (FAILED, "FAILED", false, "the operation ran and failed (`data` carries the cause)"),
        (GATED, "GATED", true, "the input layer refused the command because a dialog or another gate blocks it, exactly as it would swallow a key press"),
    ];

    /// The symbolic name of `code`.
    #[must_use]
    pub fn name(code: i64) -> &'static str {
        ALL.iter()
            .find(|(c, ..)| *c == code)
            .map_or("UNKNOWN", |(_, n, ..)| n)
    }

    /// Whether the same call may succeed later without changing its parameters.
    #[must_use]
    pub fn retryable(code: i64) -> bool {
        ALL.iter().any(|(c, _, r, _)| *c == code && *r)
    }
}

/// A JSON-RPC request. A request without `id` is a notification and is never answered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Correlation id (number or string); absent for notifications.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    /// Method name such as `input.key`.
    pub method: String,
    /// Parameters object; `null` when the method takes none.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub params: Value,
}

impl Request {
    /// Builds a request with a numeric id.
    #[must_use]
    pub fn new(id: u64, method: &str, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: method.into(),
            params,
        }
    }
}

/// The error object of a failed call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcError {
    /// One of [`code`].
    pub code: i64,
    /// Short human-readable description.
    pub message: String,
    /// Optional structured details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    /// Creates an error without `data`.
    #[must_use]
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Attaches structured details.
    #[must_use]
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// `-32602` for a bad parameter.
    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(code::INVALID_PARAMS, message)
    }
}

impl RpcError {
    /// The error as sent on the wire: `data` always is an object carrying `name` (the symbolic
    /// code) and `retryable`; other details stay as they are.
    #[must_use]
    pub fn structured(mut self) -> Self {
        let mut data = match self.data.take() {
            Some(Value::Object(m)) => m,
            Some(other) => {
                let mut m = serde_json::Map::new();
                m.insert("detail".into(), other);
                m
            }
            None => serde_json::Map::new(),
        };
        data.entry("name")
            .or_insert_with(|| json!(code::name(self.code)));
        data.entry("retryable")
            .or_insert_with(|| json!(code::retryable(self.code)));
        self.data = Some(Value::Object(data));
        self
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code {})", self.message, self.code)
    }
}

impl std::error::Error for RpcError {}

/// A JSON-RPC response: exactly one of `result` and `error` is present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// The id of the request (`null` when it could not be determined).
    pub id: Value,
    /// Success value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Failure description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl Response {
    /// A success response.
    #[must_use]
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// A failure response.
    #[must_use]
    pub fn err(id: Value, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error.structured()),
        }
    }

    /// Splits into the `Result` the caller wants.
    ///
    /// # Errors
    /// Returns the [`RpcError`] when the response carries one.
    pub fn into_result(self) -> Result<Value, RpcError> {
        match (self.result, self.error) {
            (_, Some(e)) => Err(e),
            (Some(v), None) => Ok(v),
            (None, None) => Ok(Value::Null),
        }
    }
}

/// Parses one line of server output into a [`Response`].
///
/// # Errors
/// Returns the JSON error when the line is not a response object.
pub fn parse_response_line(line: &str) -> Result<Response, serde_json::Error> {
    serde_json::from_str(line)
}

/// What one input line contains.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    /// A single request.
    Single(Request),
    /// A JSON-RPC batch; each element is a request or the error to answer with.
    Batch(Vec<Result<Request, RpcError>>),
}

fn request_from_value(v: Value) -> Result<Request, RpcError> {
    let req: Request = serde_json::from_value(v)
        .map_err(|e| RpcError::new(code::INVALID_REQUEST, format!("invalid request: {e}")))?;
    if req.jsonrpc != "2.0" {
        return Err(RpcError::new(
            code::INVALID_REQUEST,
            "`jsonrpc` must be \"2.0\"",
        ));
    }
    if !(req.params.is_null() || req.params.is_object()) {
        return Err(RpcError::invalid_params("`params` must be an object"));
    }
    Ok(req)
}

/// Parses one line of input.
///
/// # Errors
/// Returns a `-32700` error for invalid JSON and `-32600` for an empty batch or a non-request.
pub fn parse_line(line: &str) -> Result<Incoming, RpcError> {
    let value: Value = serde_json::from_str(line)
        .map_err(|e| RpcError::new(code::PARSE_ERROR, format!("parse error: {e}")))?;
    match value {
        Value::Array(items) if items.is_empty() => {
            Err(RpcError::new(code::INVALID_REQUEST, "empty batch"))
        }
        Value::Array(items) => Ok(Incoming::Batch(
            items.into_iter().map(request_from_value).collect(),
        )),
        other => request_from_value(other).map(Incoming::Single),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_and_omits_empty_params() {
        let r = Request::new(7, "ping", Value::Null);
        let line = serde_json::to_string(&r).unwrap();
        assert_eq!(line, r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#);
        assert_eq!(parse_line(&line).unwrap(), Incoming::Single(r));
    }

    #[test]
    fn error_responses_carry_code_and_data() {
        let e = RpcError::new(code::NOT_FOUND, "entity 9").with_data(json!({"id": 9}));
        let resp = Response::err(json!(1), e.clone());
        let line = serde_json::to_string(&resp).unwrap();
        let back: Response = serde_json::from_str(&line).unwrap();
        // The wire form adds the symbolic name and the retry hint; the caller's data stays.
        let got = back.into_result().unwrap_err();
        assert_eq!(got, e.structured());
        let data = got.data.expect("data");
        assert_eq!(data["id"], 9);
        assert_eq!(data["name"], "NOT_FOUND");
        assert_eq!(data["retryable"], false);
        let gated = RpcError::new(code::GATED, "blocked").structured();
        assert_eq!(gated.data.expect("data")["retryable"], true);
    }

    #[test]
    fn invalid_input_maps_to_the_right_codes() {
        assert_eq!(parse_line("{nope").unwrap_err().code, code::PARSE_ERROR);
        assert_eq!(parse_line("[]").unwrap_err().code, code::INVALID_REQUEST);
        assert_eq!(parse_line("5").unwrap_err().code, code::INVALID_REQUEST);
        let wrong = r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#;
        assert_eq!(parse_line(wrong).unwrap_err().code, code::INVALID_REQUEST);
        let positional = r#"{"jsonrpc":"2.0","id":1,"method":"x","params":[1]}"#;
        assert_eq!(
            parse_line(positional).unwrap_err().code,
            code::INVALID_PARAMS
        );
    }

    #[test]
    fn batches_keep_per_element_errors() {
        let line = r#"[{"jsonrpc":"2.0","id":1,"method":"ping"}, 3]"#;
        let Incoming::Batch(items) = parse_line(line).unwrap() else {
            panic!("expected batch");
        };
        assert!(items[0].is_ok());
        assert_eq!(items[1].as_ref().unwrap_err().code, code::INVALID_REQUEST);
    }

    #[test]
    fn notifications_have_no_id() {
        let line = r#"{"jsonrpc":"2.0","method":"sim.pause"}"#;
        let Incoming::Single(r) = parse_line(line).unwrap() else {
            panic!("expected single");
        };
        assert!(r.id.is_none());
    }
}
