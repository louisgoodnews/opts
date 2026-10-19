//! The command table of the control protocol and the translation of command-line words into
//! JSON parameters.
//!
//! Parameters are declared with a tiny notation: `name:type[@position][?][=alias,alias]` where `type`
//! is `s` (string), `i` (integer), `f` (float), `b` (bool) or `j` (JSON value), `@n` makes the
//! parameter the n-th positional word, and `?` marks it optional.

use serde_json::{json, Map, Value};

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Str,
    Int,
    Float,
    Bool,
    Json,
}

struct ParamDecl<'a> {
    name: &'a str,
    kind: Kind,
    position: Option<usize>,
    optional: bool,
    aliases: Vec<&'a str>,
    values: Vec<&'a str>,
}

fn parse_decls(decl: &str) -> Vec<ParamDecl<'_>> {
    decl.split_whitespace()
        .filter_map(|word| {
            let (head, aliases) = word.split_once('=').unwrap_or((word, ""));
            let (name, rest) = head.split_once(':')?;
            let optional = rest.contains('?');
            let position = rest
                .split_once('@')
                .map(|(_, tail)| {
                    tail.trim_end_matches('?')
                        .parse::<usize>()
                        .unwrap_or(usize::MAX)
                })
                .filter(|p| *p != usize::MAX);
            let values: Vec<&str> = rest
                .split_once('[')
                .and_then(|(_, t)| t.split_once(']'))
                .map(|(v, _)| v.split('|').filter(|x| !x.is_empty()).collect())
                .unwrap_or_default();
            let kind = match rest.chars().next()? {
                's' => Kind::Str,
                'i' => Kind::Int,
                'f' => Kind::Float,
                'b' => Kind::Bool,
                'j' => Kind::Json,
                _ => return None,
            };
            Some(ParamDecl {
                name,
                kind,
                position,
                optional,
                aliases: aliases.split(',').filter(|a| !a.is_empty()).collect(),
                values,
            })
        })
        .collect()
}

fn coerce(kind: Kind, name: &str, raw: &str) -> Result<Value, String> {
    let bad = |what: &str| format!("parameter `{name}` must be {what}, got `{raw}`");
    match kind {
        Kind::Str => Ok(json!(raw)),
        Kind::Int => raw
            .parse::<i64>()
            .map(|v| json!(v))
            .map_err(|_| bad("an integer")),
        Kind::Float => raw
            .parse::<f64>()
            .map(|v| json!(v))
            .map_err(|_| bad("a number")),
        Kind::Bool => match raw {
            "true" | "1" | "on" | "yes" => Ok(json!(true)),
            "false" | "0" | "off" | "no" => Ok(json!(false)),
            _ => Err(bad("true or false")),
        },
        // A word that is not JSON (`drive`, `toggle`) is a string, so `drive.set gear drive` works.
        Kind::Json => Ok(serde_json::from_str(raw).unwrap_or_else(|_| json!(raw))),
    }
}

/// Builds the `params` object of `spec` from command-line words.
///
/// Words are positional values, `--name value`, `--name=value` or `name=value`; `--json '{...}'`
/// supplies a whole parameter object (explicit words override it).
///
/// # Errors
/// A readable message for unknown flags, bad values and missing required parameters.
pub fn build_params(spec: &CommandSpec, words: &[String]) -> Result<Value, String> {
    let decls = parse_decls(spec.params);
    let mut out = Map::new();
    let mut positional = 0usize;
    let mut i = 0;
    while i < words.len() {
        let w = &words[i];
        let (key, inline): (Option<&str>, Option<&str>) = if let Some(flag) = w.strip_prefix("--") {
            match flag.split_once('=') {
                Some((k, v)) => (Some(k), Some(v)),
                None => (Some(flag), None),
            }
        } else if let Some((k, v)) = w.split_once('=').filter(|(k, _)| is_name(k)) {
            (Some(k), Some(v))
        } else {
            (None, None)
        };
        if let Some(key) = key {
            if key == "json" {
                let raw = match inline {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        words.get(i).cloned().ok_or("--json needs a value")?
                    }
                };
                let v: Value = serde_json::from_str(&raw).map_err(|e| format!("--json: {e}"))?;
                let Value::Object(map) = v else {
                    return Err("--json must be an object".into());
                };
                for (k, v) in map {
                    out.entry(k).or_insert(v);
                }
                i += 1;
                continue;
            }
            let key = key.replace('-', "_");
            if spec.method.is_empty() {
                // Unknown (newer) method: take the pair as written; JSON if it parses, text otherwise.
                let raw = match inline {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        words
                            .get(i)
                            .cloned()
                            .ok_or_else(|| format!("`--{key}` needs a value"))?
                    }
                };
                let value = serde_json::from_str(&raw).unwrap_or_else(|_| json!(raw));
                out.insert(key, value);
                i += 1;
                continue;
            }
            let decl = decls
                .iter()
                .find(|d| d.name == key || d.aliases.iter().any(|a| a.replace('-', "_") == key))
                .ok_or_else(|| format!("`{}` has no parameter `{key}`", spec.method))?;
            let raw = match inline {
                Some(v) => v.to_string(),
                None if decl.kind == Kind::Bool
                    && words.get(i + 1).is_none_or(|n| n.starts_with("--")) =>
                {
                    "true".to_string()
                }
                None => {
                    i += 1;
                    words
                        .get(i)
                        .cloned()
                        .ok_or_else(|| format!("`--{key}` needs a value"))?
                }
            };
            out.insert(decl.name.to_string(), coerce(decl.kind, decl.name, &raw)?);
        } else {
            let decl = decls
                .iter()
                .find(|d| d.position == Some(positional))
                .ok_or_else(|| format!("`{}` takes no value `{w}`", spec.method))?;
            out.insert(decl.name.to_string(), coerce(decl.kind, decl.name, w)?);
            positional += 1;
        }
        i += 1;
    }
    if let Some(missing) = decls
        .iter()
        .find(|d| !d.optional && !out.contains_key(d.name))
    {
        return Err(format!("`{}` needs `{}`", spec.method, missing.name));
    }
    Ok(if out.is_empty() {
        Value::Null
    } else {
        Value::Object(out)
    })
}

fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests;
