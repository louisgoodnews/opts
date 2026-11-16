//! The JSON-RPC request, error and response objects of the control protocol.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::code;

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
