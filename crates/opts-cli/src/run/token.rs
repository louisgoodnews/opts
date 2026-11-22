//! The client's token: from `OPTS_CLI_TOKEN` or `--token-file`, never from the command line
//! (ISS-004), and the refusal of a token given as an argument of `auth`.

use super::Cli;
use crate::token::from_sources;

impl Cli {
    /// Refuses `--token` and reads the token from its sources into [`Cli::token`].
    ///
    /// # Errors
    /// `--token` was given, both sources are set, or the file cannot be read.
    pub fn resolve_token(&mut self) -> Result<(), String> {
        if self.token_argv.is_some() {
            return Err(
                "--token is refused: the token is not read from the command line. \
                 Set OPTS_CLI_TOKEN or pass --token-file <PATH>"
                    .into(),
            );
        }
        self.token = from_sources(self.token_file.as_deref())?;
        Ok(())
    }
}

/// The refusal of `auth` with an argument: the token would end up in the shell history.
pub(super) fn auth_argument_error(method: &str, rest: &[String]) -> Option<String> {
    (method == "auth" && !rest.is_empty()).then(|| {
        "auth takes no argument: the token comes from OPTS_CLI_TOKEN or --token-file".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn the_old_token_flag_is_refused() {
        let mut cli = Cli::try_parse_from(["opts-cli", "--token", "x", "ping"]).expect("parse");
        let err = cli.resolve_token().expect_err("argv token refused");
        assert!(err.contains("--token is refused"), "{err}");
        assert_eq!(cli.token, None);
    }

    #[test]
    fn auth_with_an_argument_is_refused_and_without_one_allowed() {
        assert!(auth_argument_error("auth", &["secret".to_string()]).is_some());
        assert!(auth_argument_error("auth", &[]).is_none());
        assert!(auth_argument_error("ping", &["x".to_string()]).is_none());
    }
}
