//! Error codes of the control protocol: the JSON-RPC reserved range and the application range.

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
