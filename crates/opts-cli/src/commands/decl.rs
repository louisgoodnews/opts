//! Parameter declarations of the command table: the `name:type[@position][?][=alias]` notation
//! parsed into [`ParamDecl`]s, and the coercion of raw words into typed JSON values.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Kind {
    Str,
    Int,
    Float,
    Bool,
    Json,
}

pub(super) struct ParamDecl<'a> {
    pub(super) name: &'a str,
    pub(super) kind: Kind,
    pub(super) position: Option<usize>,
    pub(super) optional: bool,
    pub(super) aliases: Vec<&'a str>,
    pub(super) values: Vec<&'a str>,
}

pub(super) fn parse_decls(decl: &str) -> Vec<ParamDecl<'_>> {
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

pub(super) fn coerce(kind: Kind, name: &str, raw: &str) -> Result<Value, String> {
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

