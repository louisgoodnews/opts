//! The `help` command of the interactive mode.

use crate::commands::{find, COMMANDS};
use crate::run::out;

pub(super) fn show_help(topic: &[String]) {
    match topic.first().and_then(|t| find(t)) {
        Some(c) => out(&format!(
            "{}\n  {}\n  parameters: {}",
            c.method,
            c.summary,
            if c.params.is_empty() { "-" } else { c.params }
        )),
        None => {
            out("commands (Tab completes, `? <command>` describes one):");
            for c in COMMANDS {
                out(&format!("  {:<22} {}", c.method, c.summary));
            }
            out("exit with `exit` or Ctrl-D");
        }
    }
}
