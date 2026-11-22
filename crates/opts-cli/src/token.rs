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
    let file_text = file.map(read_token_file).transpose()?;
    combine(file_text, std::env::var_os(TOKEN_ENV))
}

/// Reads the first line of a token file. On Unix a file that group or others can read
/// (`mode & 0o077 != 0`) is refused, so other local accounts cannot take the token (ISS-004).
fn read_token_file(path: &Path) -> Result<String, String> {
    refuse_shared_file(path)?;
    std::fs::read_to_string(path)
        .map(|t| t.trim_end_matches(['\n', '\r']).to_string())
        .map_err(|e| format!("cannot read the token file {}: {e}", path.display()))
}

/// Refuses a file whose permission bits give group or others any access.
#[cfg(unix)]
fn refuse_shared_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("cannot read the token file {}: {e}", path.display()))?;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(format!(
            "the token file {} is readable by group or others (mode {mode:o}); make it 0600",
            path.display()
        ));
    }
    Ok(())
}

/// Other platforms have no mode bits to check here.
#[cfg(not(unix))]
fn refuse_shared_file(_path: &Path) -> Result<(), String> {
    Ok(())
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

    /// A token file with `mode`, in a folder next to the test binary (never `/tmp`).
    #[cfg(unix)]
    fn token_file(name: &str, mode: u32) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let exe = std::env::current_exe().expect("test binary path");
        let dir = exe
            .parent()
            .expect("folder of the binary")
            .join("token-files");
        std::fs::create_dir_all(&dir).expect("folder");
        let path = dir.join(format!("{name}-{}", std::process::id()));
        std::fs::write(&path, "0123456789abcdef0123456789abcdef\n").expect("write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("mode");
        path
    }

    #[cfg(unix)]
    #[test]
    fn a_0600_token_file_is_accepted() {
        let path = token_file("accept-0600", 0o600);
        let got = read_token_file(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(got, Ok("0123456789abcdef0123456789abcdef".into()));
    }

    #[cfg(unix)]
    #[test]
    fn a_0644_token_file_is_refused() {
        let path = token_file("refuse-0644", 0o644);
        let got = read_token_file(&path);
        let _ = std::fs::remove_file(&path);
        let err = got.unwrap_err();
        assert!(
            err.contains("group or others") && err.contains("0600"),
            "{err}"
        );
    }
}
