//! Where the control token comes from: the `OPTS_CLI_TOKEN` environment variable or a file named by
//! a path (`--cli-token-file` in the game, `--token-file` in `opts-cli`). Never the command line:
//! argv is visible to other users and stays in the shell history (ISS-004).

use std::ffi::OsString;
use std::path::Path;

use serde_json::{json, Value};

/// Environment variable that holds the control token.
pub const TOKEN_ENV: &str = "OPTS_CLI_TOKEN";

/// The token from the environment or from `file`; `None` when neither is set.
///
/// # Errors
/// Both sources are given, the file cannot be read, or the environment value is not UTF-8.
pub fn from_sources(file: Option<&Path>) -> Result<Option<String>, String> {
    let file_text = file
        .map(|p| {
            std::fs::read_to_string(p)
                .map(|t| t.trim_end_matches(['\n', '\r']).to_string())
                .map_err(|e| format!("cannot read the token file {}: {e}", p.display()))
        })
        .transpose()?;
    combine(file_text, std::env::var_os(TOKEN_ENV))
}

/// Picks the single source that is set; setting both is an error.
fn combine(file_text: Option<String>, env: Option<OsString>) -> Result<Option<String>, String> {
    match (file_text, env) {
        (Some(_), Some(_)) => Err(format!("set either {TOKEN_ENV} or a token file, not both")),
        (Some(text), None) => Ok(Some(text)),
        (None, Some(value)) => value
            .into_string()
            .map(Some)
            .map_err(|_| format!("{TOKEN_ENV} is not valid UTF-8")),
        (None, None) => Ok(None),
    }
}

/// `auth` without a token argument sends the configured token; other methods are unchanged.
pub fn with_auth_token(method: &str, params: Value, token: Option<&str>) -> Value {
    let Some(token) = token.filter(|_| method == "auth") else {
        return params;
    };
    let mut obj = match params {
        Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };
    obj.entry("token").or_insert_with(|| json!(token));
    Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_is_the_source_when_the_environment_is_empty() {
        assert_eq!(combine(Some("abc".into()), None), Ok(Some("abc".into())));
    }

    #[test]
    fn the_environment_is_the_source_without_a_file() {
        let env = Some(OsString::from("xyz"));
        assert_eq!(combine(None, env), Ok(Some("xyz".into())));
        assert_eq!(combine(None, None), Ok(None));
    }

    #[test]
    fn both_sources_at_once_are_refused() {
        let err = combine(Some("a".into()), Some(OsString::from("b"))).unwrap_err();
        assert!(err.contains("not both"), "{err}");
    }
}
