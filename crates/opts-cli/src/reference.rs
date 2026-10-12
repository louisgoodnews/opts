//! The generated reference page `docs/src/reference/control-protocol.md`.
//!
//! It is produced from the command table ([`crate::commands::COMMANDS`]) that also feeds the
//! server's `help` and `commands`, so the page, discovery and the client cannot drift apart. The
//! test `reference_page_is_up_to_date` fails when the file is stale.

use std::fmt::Write as _;

use crate::commands::{groups, CommandSpec, COMMANDS};
use crate::protocol::{code, PROTOCOL_VERSION};

fn args_cell(c: &CommandSpec) -> String {
    let args = c.args_schema();
    let Some(list) = args.as_array().filter(|l| !l.is_empty()) else {
        return "-".into();
    };
    list.iter()
        .map(|a| {
            let name = a["name"].as_str().unwrap_or("");
            let ty = a["type"].as_str().unwrap_or("");
            let opt = if a["required"].as_bool() == Some(true) {
                ""
            } else {
                "?"
            };
            let values = a["values"]
                .as_array()
                .map(|v| {
                    format!(
                        " = {}",
                        v.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(" \\| ")
                    )
                })
                .unwrap_or_default();
            format!("`{name}{opt}` ({ty}{values})")
        })
        .collect::<Vec<_>>()
        .join("<br>")
}

/// The reference page in Markdown.
#[must_use]
pub fn markdown() -> String {
    let mut out = String::new();
    out.push_str(
        "<!-- Generated from the command table of opts-cli (crates/opts-cli/src/commands.rs) by the test \
         `reference_page_is_up_to_date`; regenerate with `UPDATE_CONTROL_DOC=1 cargo test -p opts-cli reference`. -->\n\n",
    );
    out.push_str("# Control Protocol Reference\n\n");
    let _ = write!(
        out,
        "Protocol version **{PROTOCOL_VERSION}**. Every command below is declared once in the command table; the same table \
         answers `help <command>` and `commands` at runtime, drives the `opts-cli` argument parser and generates this page. \
         Transport, message format, gating and worked sessions are in [the architecture page](../architecture/control-protocol.md).\n\n\
         *Debug* commands deliberately skip the input layer and its gates (they act on the world directly); every other command goes \
         through the same messages, keys and systems a person would use, so a dialog that swallows a key press also refuses the \
         command (`1009`).\n\n"
    );
    out.push_str("## Errors\n\nEvery error carries `data.name` (the symbolic code) and `data.retryable`.\n\n| Code | Name | Retry | Meaning |\n| ---: | :--- | :--- | :--- |\n");
    for (c, n, r, d) in code::ALL {
        let _ = writeln!(
            out,
            "| {c} | `{n}` | {} | {d} |",
            if *r { "later" } else { "no" }
        );
    }
    for g in groups() {
        let _ = write!(out, "\n## Group `{g}`\n\n");
        out.push_str("| Command | Arguments | Since | Summary |\n| :--- | :--- | :--- | :--- |\n");
        for c in COMMANDS.iter().filter(|c| c.group == g) {
            let debug = if c.debug { " *(debug)*" } else { "" };
            let name = if c.result.is_empty() && c.doc.is_empty() {
                format!("`{}`", c.method)
            } else {
                format!("[`{}`](#{})", c.method, anchor(c.method))
            };
            let _ = writeln!(
                out,
                "| {name}{debug} | {args} | {since} | {summary} |",
                args = args_cell(c),
                since = c.since,
                summary = c.summary,
            );
        }
        for c in COMMANDS
            .iter()
            .filter(|c| c.group == g && (!c.result.is_empty() || !c.doc.is_empty()))
        {
            let _ = write!(out, "\n### `{}`\n\n", c.method);
            let _ = writeln!(out, "{}.", c.summary.trim_end_matches('.'));
            if !c.result.is_empty() {
                let _ = write!(out, "\n**Result:** {}\n", c.result);
            }
            if !c.doc.is_empty() {
                let _ = write!(out, "\n{}\n", c.doc);
            }
        }
    }
    out
}

fn anchor(method: &str) -> String {
    method.replace('.', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_page_is_up_to_date() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/src/reference/control-protocol.md");
        let want = markdown();
        if std::env::var_os("UPDATE_CONTROL_DOC").is_some() {
            std::fs::write(&path, &want).expect("write reference page");
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            have, want,
            "docs/src/reference/control-protocol.md is stale: run `UPDATE_CONTROL_DOC=1 cargo test -p opts-cli reference`"
        );
    }

    #[test]
    fn every_command_is_documented_and_groups_are_known() {
        for c in COMMANDS {
            assert!(!c.summary.is_empty(), "{} needs a summary", c.method);
            assert!(c.since.starts_with("1."), "{} needs `since`", c.method);
        }
        let json = COMMANDS[0].descriptor_json();
        assert_eq!(json["method"], "ping");
    }

    #[test]
    fn enum_values_are_part_of_the_schema() {
        let spec = crate::commands::find("ui.scroll").expect("ui.scroll");
        let args = spec.args_schema();
        let to = args
            .as_array()
            .and_then(|a| a.iter().find(|x| x["name"] == "to"))
            .expect("`to`");
        assert_eq!(to["values"], serde_json::json!(["top", "bottom", "left", "right"]));
    }
}
