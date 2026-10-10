//! Entry point logic of the `opts-cli` binary: argument handling, command execution, batch mode.

use std::io::Read;
use std::time::Duration;

use clap::Parser;
use serde_json::Value;

use crate::client::{Client, ClientConfig, ClientError};
use crate::commands::{build_params, find, COMMANDS};
use crate::output::{render_error, render_result, OutputMode};
use crate::protocol::{RpcError, DEFAULT_PORT};

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

/// Methods that legitimately take long (they reply after N frames or after loading).
fn is_slow(method: &str) -> bool {
    matches!(
        method,
        "sim.step" | "scene.load_map" | "scene.unload" | "vehicle.set"
    )
}

/// Runs one command from words and prints the outcome; returns the process exit code.
pub fn execute(client: &mut Client, mode: OutputMode, words: &[String]) -> u8 {
    let Some((method, rest)) = words.split_first() else {
        eprintln!("no command given; try `opts-cli list`");
        return 1;
    };
    let params = match find(method) {
        Some(spec) => build_params(spec, rest),
        None => build_params(&crate::commands::generic_spec(), rest),
    };
    let params = match params {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
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
            eprintln!("{}", render_error(mode, &e));
            1
        }
        Err(e) => {
            eprintln!("{e}");
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

fn run_batch(client: &mut Client, mode: OutputMode) -> u8 {
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("cannot read stdin: {e}");
        return 1;
    }
    let value: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("stdin is not JSON: {e}");
            return 1;
        }
    };
    let Some(items) = value.as_array() else {
        eprintln!("batch input must be a JSON array of {{\"method\", \"params\"}} objects");
        return 1;
    };
    let mut commands = Vec::new();
    for item in items {
        let Some(method) = item.get("method").and_then(Value::as_str) else {
            eprintln!("every batch entry needs a `method`");
            return 1;
        };
        commands.push((
            method.to_string(),
            item.get("params").cloned().unwrap_or(Value::Null),
        ));
    }
    match client.batch(&commands, false) {
        Ok(results) => print_batch(mode, &commands, &results),
        Err(ClientError::Rpc(e)) => {
            eprintln!("{}", render_error(mode, &e));
            1
        }
        Err(e) => {
            eprintln!("{e}");
            e.exit_code()
        }
    }
}

fn print_batch(
    mode: OutputMode,
    commands: &[(String, Value)],
    results: &[Result<Value, RpcError>],
) -> u8 {
    let mut failed = false;
    if mode == OutputMode::Json {
        let all: Vec<Value> = results
            .iter()
            .map(|r| match r {
                Ok(v) => serde_json::json!({ "result": v }),
                Err(e) => serde_json::json!({ "error": e }),
            })
            .collect();
        out(&serde_json::to_string_pretty(&all).unwrap_or_default());
        failed = results.iter().any(Result::is_err);
    } else {
        for ((method, _), r) in commands.iter().zip(results) {
            match r {
                Ok(v) => {
                    if let Some(t) = render_result(mode, method, v) {
                        out(&format!("[{method}] {t}"));
                    }
                }
                Err(e) => {
                    failed = true;
                    eprintln!("[{method}] {}", render_error(mode, e));
                }
            }
        }
    }
    u8::from(failed)
}

/// Runs the CLI and returns the exit code.
pub fn run(cli: &Cli) -> u8 {
    let mode = cli.mode();
    let first = cli.words.first().map(String::as_str);
    match first {
        Some("list" | "commands") => {
            print_list();
            0
        }
        Some("repl") => crate::repl::run_repl(Client::new(cli.config()), mode),
        Some("batch") => run_batch(&mut Client::new(cli.config()), mode),
        Some("watch") => {
            eprintln!("`watch` needs the event stream, which protocol 1.0 does not have yet");
            1
        }
        _ => execute(&mut Client::new(cli.config()), mode, &cli.words),
    }
}
