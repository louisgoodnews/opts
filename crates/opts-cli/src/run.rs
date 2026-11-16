//! Entry point logic of the `opts-cli` binary: argument handling, command execution, batch mode.

use std::time::Duration;

use clap::Parser;

use crate::client::{Client, ClientConfig, ClientError};
use crate::commands::{build_params, find, COMMANDS};
use crate::output::{render_error, render_result, OutputMode};
use crate::protocol::DEFAULT_PORT;

mod batch;
mod method;
use batch::run_batch;
use method::{apply_cli_defaults, is_slow, split_method};

/// Remote control of a running OPTS instance.
///
/// Exit codes: 0 success, 1 command error, 2 connection error, 3 protocol error.
#[derive(Parser, Debug, Clone)]
#[command(name = "opts-cli", version, about, trailing_var_arg = true)]
pub struct Cli {
    /// Host of the control server.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Port of the control server.
    #[arg(long, default_value_t = DEFAULT_PORT)]
    pub port: u16,
    /// Token for servers started with `--cli-token`.
    #[arg(long)]
    pub token: Option<String>,
    /// Print results as JSON.
    #[arg(long)]
    pub json: bool,
    /// Print nothing on success.
    #[arg(long, short)]
    pub quiet: bool,
    /// Connect/read timeout in milliseconds.
    #[arg(long, default_value_t = 10_000)]
    pub timeout_ms: u64,
    /// Command and its arguments: `state`, `input.key W --duration 2000`, `batch`, `repl`, `list`.
    #[arg(allow_hyphen_values = true)]
    pub words: Vec<String>,
}

impl Cli {
    /// The output mode selected by the flags.
    #[must_use]
    pub fn mode(&self) -> OutputMode {
        if self.json {
            OutputMode::Json
        } else if self.quiet {
            OutputMode::Quiet
        } else {
            OutputMode::Human
        }
    }

    /// The connection settings selected by the flags.
    #[must_use]
    pub fn config(&self) -> ClientConfig {
        ClientConfig {
            host: self.host.clone(),
            port: self.port,
            token: self.token.clone(),
            timeout: Duration::from_millis(self.timeout_ms),
        }
    }
}

/// Prints a line to stdout and ignores a closed pipe (`opts-cli … | head`).
pub fn out(text: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stdout(), "{text}");
}


/// Runs one command from words and prints the outcome; returns the process exit code.
pub fn execute(client: &mut Client, mode: OutputMode, words: &[String]) -> u8 {
    let Some((method, rest)) = split_method(words) else {
        errln!("no command given; try `opts-cli list`");
        return 1;
    };
    let method = method.as_str();
    let params = match find(method) {
        Some(spec) => build_params(spec, rest),
        None => build_params(&crate::commands::generic_spec(), rest),
    };
    let params = match params {
        Ok(p) => p,
        Err(e) => {
            errln!("{e}");
            return 1;
        }
    };
    let mut params = params;
    apply_cli_defaults(method, &mut params);
    if is_slow(method) {
        client.set_read_timeout(Duration::from_secs(120));
    }
    match client.call(method, params) {
        Ok(result) => {
            if let Some(text) = render_result(mode, method, &result) {
                out(&text);
            }
            0
        }
        Err(ClientError::Rpc(e)) => {
            errln!("{}", render_error(mode, &e));
            1
        }
        Err(e) => {
            errln!("{e}");
            e.exit_code()
        }
    }
}

fn print_list() {
    let mut group = "";
    for c in COMMANDS {
        if c.group != group {
            group = c.group;
            out(&format!("\n{group}"));
        }
        out(&format!("  {:<22} {}", c.method, c.summary));
    }
}

/// Runs the CLI and returns the exit code.
pub fn run(cli: &Cli) -> u8 {
    let mode = cli.mode();
    let first = cli.words.first().map(String::as_str);
    match first {
        Some("list") => {
            print_list();
            0
        }
        Some("repl") => crate::repl::run_repl(Client::new(cli.config()), mode),
        Some("batch") => run_batch(&mut Client::new(cli.config()), mode),
        Some("watch") => {
            errln!("`watch` needs the event stream, which protocol 1.0 does not have yet");
            1
        }
        _ => execute(&mut Client::new(cli.config()), mode, &cli.words),
    }
}

#[cfg(test)]
mod tests;
