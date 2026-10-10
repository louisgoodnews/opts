//! Typed access to JSON parameters with protocol-conformant errors (`-32602`).

use opts_cli::protocol::RpcError;
use serde_json::Value;

fn missing(name: &str) -> RpcError {
    RpcError::invalid_params(format!("missing parameter `{name}`"))
}

fn wrong(name: &str, want: &str) -> RpcError {
    RpcError::invalid_params(format!("parameter `{name}` must be {want}"))
}

/// A required string.
///
/// # Errors
/// `-32602` when absent or not a string.
pub fn str_param<'a>(p: &'a Value, name: &str) -> Result<&'a str, RpcError> {
    p.get(name)
        .ok_or_else(|| missing(name))?
        .as_str()
        .ok_or_else(|| wrong(name, "a string"))
}

/// An optional string.
///
/// # Errors
/// `-32602` when present but not a string.
pub fn opt_str<'a>(p: &'a Value, name: &str) -> Result<Option<&'a str>, RpcError> {
    match p.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_str().map(Some).ok_or_else(|| wrong(name, "a string")),
    }
}

/// A required number.
///
/// # Errors
/// `-32602` when absent or not numeric.
pub fn f64_param(p: &Value, name: &str) -> Result<f64, RpcError> {
    p.get(name)
        .ok_or_else(|| missing(name))?
        .as_f64()
        .ok_or_else(|| wrong(name, "a number"))
}

/// An optional number.
///
/// # Errors
/// `-32602` when present but not numeric.
pub fn opt_f64(p: &Value, name: &str) -> Result<Option<f64>, RpcError> {
    match p.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().map(Some).ok_or_else(|| wrong(name, "a number")),
    }
}

/// A required non-negative integer.
///
/// # Errors
/// `-32602` when absent or not an unsigned integer.
pub fn u64_param(p: &Value, name: &str) -> Result<u64, RpcError> {
    p.get(name)
        .ok_or_else(|| missing(name))?
        .as_u64()
        .ok_or_else(|| wrong(name, "a non-negative integer"))
}

/// An optional non-negative integer.
///
/// # Errors
/// `-32602` when present but not an unsigned integer.
pub fn opt_u64(p: &Value, name: &str) -> Result<Option<u64>, RpcError> {
    match p.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .ok_or_else(|| wrong(name, "a non-negative integer")),
    }
}

/// An optional boolean.
///
/// # Errors
/// `-32602` when present but not a boolean.
pub fn opt_bool(p: &Value, name: &str) -> Result<Option<bool>, RpcError> {
    match p.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_bool()
            .map(Some)
            .ok_or_else(|| wrong(name, "true or false")),
    }
}

/// A required boolean.
///
/// # Errors
/// `-32602` when absent or not a boolean.
pub fn bool_param(p: &Value, name: &str) -> Result<bool, RpcError> {
    opt_bool(p, name)?.ok_or_else(|| missing(name))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn required_and_optional_parameters() {
        let p = json!({"key": "W", "n": 3, "x": 1.5, "on": true});
        assert_eq!(str_param(&p, "key").unwrap(), "W");
        assert_eq!(u64_param(&p, "n").unwrap(), 3);
        assert_eq!(f64_param(&p, "x").unwrap(), 1.5);
        assert!(bool_param(&p, "on").unwrap());
        assert_eq!(opt_str(&p, "none").unwrap(), None);
        assert_eq!(opt_u64(&p, "n").unwrap(), Some(3));
    }

    #[test]
    fn errors_are_invalid_params() {
        let p = json!({"n": "three", "k": -1});
        assert!(str_param(&p, "missing")
            .unwrap_err()
            .message
            .contains("missing"));
        assert!(u64_param(&p, "n")
            .unwrap_err()
            .message
            .contains("non-negative"));
        assert!(u64_param(&p, "k").is_err());
        assert_eq!(f64_param(&Value::Null, "x").unwrap_err().code, -32602);
    }
}
