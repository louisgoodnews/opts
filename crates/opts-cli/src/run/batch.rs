//! Batch mode of the CLI: command lines read from standard input, one result line per command.

use std::io::Read;

use serde_json::Value;

use crate::client::{Client, ClientError};
use crate::output::{render_error, render_result, OutputMode};
use crate::protocol::RpcError;
use crate::run::out;

pub(super) fn run_batch(client: &mut Client, mode: OutputMode) -> u8 {
    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        errln!("cannot read stdin: {e}");
        return 1;
    }
    let value: Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => {
            errln!("stdin is not JSON: {e}");
            return 1;
        }
    };
    let Some(items) = value.as_array() else {
        errln!("batch input must be a JSON array of {{\"method\", \"params\"}} objects");
        return 1;
    };
    let mut commands = Vec::new();
    for item in items {
        let Some(method) = item.get("method").and_then(Value::as_str) else {
            errln!("every batch entry needs a `method`");
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
            errln!("{}", render_error(mode, &e));
            1
        }
        Err(e) => {
            errln!("{e}");
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
                    errln!("[{method}] {}", render_error(mode, e));
                }
            }
        }
    }
    u8::from(failed)
}
