//! The command table of the control protocol and the translation of command-line words into
//! JSON parameters.
//!
//! Parameters are declared with a tiny notation: `name:type[@position][?][=alias,alias]` where `type`
//! is `s` (string), `i` (integer), `f` (float), `b` (bool) or `j` (JSON value), `@n` makes the
//! parameter the n-th positional word, and `?` marks it optional.

use serde_json::{json, Value};

mod table;

/// One protocol method.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    /// Method name sent to the server.
    pub method: &'static str,
    /// Group shown by `list`.
    pub group: &'static str,
    /// One-line description.
    pub summary: &'static str,
    /// Parameter declaration (see the module docs); empty for none.
    pub params: &'static str,
    /// Protocol version that introduced the method (`1.0`, `1.1`, ...).
    pub since: &'static str,
    /// What the result contains (one line); empty when it is only `{ok:true}`.
    pub result: &'static str,
    /// Longer description: semantics, gating, caveats; empty when the summary says it all.
    pub doc: &'static str,
    /// A debug command that deliberately skips the input layer and its gates (documented as such).
    pub debug: bool,
}

impl CommandSpec {
    /// Sets the version that introduced the method.
    #[must_use]
    pub const fn since(mut self, v: &'static str) -> Self {
        self.since = v;
        self
    }

    /// Sets the result description.
    #[must_use]
    pub const fn result(mut self, r: &'static str) -> Self {
        self.result = r;
        self
    }

    /// Sets the long description.
    #[must_use]
    pub const fn doc(mut self, d: &'static str) -> Self {
        self.doc = d;
        self
    }

    /// Marks a debug command that bypasses the input layer.
    #[must_use]
    pub const fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// The parameters as `{name, type, required, position, aliases, values}` objects.
    #[must_use]
    pub fn args_schema(&self) -> Value {
        Value::Array(
            parse_decls(self.params)
                .iter()
                .map(|d| {
                    let ty = match d.kind {
                        Kind::Str => "string",
                        Kind::Int => "integer",
                        Kind::Float => "number",
                        Kind::Bool => "boolean",
                        Kind::Json => "json",
                    };
                    let mut o = json!({
                        "name": d.name,
                        "type": ty,
                        "required": !d.optional,
                        "position": d.position,
                        "aliases": d.aliases,
                    });
                    if !d.values.is_empty() {
                        o["values"] = json!(d.values);
                    }
                    o
                })
                .collect(),
        )
    }

    /// The full descriptor as sent by `help` and `commands`.
    #[must_use]
    pub fn descriptor_json(&self) -> Value {
        json!({
            "method": self.method,
            "group": self.group,
            "summary": self.summary,
            "since": self.since,
            "args": self.args_schema(),
            "result": self.result,
            "doc": self.doc,
            "debug": self.debug,
        })
    }
}

const fn c(
    method: &'static str,
    group: &'static str,
    summary: &'static str,
    params: &'static str,
) -> CommandSpec {
    CommandSpec {
        method,
        group,
        summary,
        params,
        since: "1.0",
        result: "",
        doc: "",
        debug: false,
    }
}

/// Every method of protocol 1.x.
pub const COMMANDS: &[CommandSpec] = &table::join();

/// A spec for methods this client does not know (a newer server): any `name=value` or `--name value`
/// pair becomes a string/number/bool/JSON parameter.
#[must_use]
pub fn generic_spec() -> CommandSpec {
    c("", "", "", "")
}

/// The group names in table order, without repeats.
#[must_use]
pub fn groups() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for c in COMMANDS {
        if !out.contains(&c.group) {
            out.push(c.group);
        }
    }
    out
}

/// Looks a method up by name.
#[must_use]
pub fn find(method: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|c| c.method == method)
}

/// Method names starting with `prefix` (for completion).
pub fn completions(prefix: &str) -> impl Iterator<Item = &'static str> + '_ {
    COMMANDS
        .iter()
        .map(|c| c.method)
        .filter(move |m| m.starts_with(prefix))
}

mod build;
mod decl;

pub use build::build_params;
use decl::{parse_decls, Kind};

#[cfg(test)]
mod tests;
