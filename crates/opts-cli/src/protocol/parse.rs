//! Parsing of input lines (requests, batches) and of server output lines (responses).

use serde_json::Value;

use super::code;
use super::message::{Request, RpcError, Response};

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
