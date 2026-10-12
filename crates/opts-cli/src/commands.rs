//! The command table of the control protocol and the translation of command-line words into
//! JSON parameters.
//!
//! Parameters are declared with a tiny notation: `name:type[@position][?][=alias,alias]` where `type`
//! is `s` (string), `i` (integer), `f` (float), `b` (bool) or `j` (JSON value), `@n` makes the
//! parameter the n-th positional word, and `?` marks it optional.

use serde_json::{json, Map, Value};

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
    }
}

/// Every method of protocol 1.x.
pub const COMMANDS: &[CommandSpec] = &[
    c("ping", "meta", "Check that the server answers", ""),
    c("version", "meta", "App and protocol version", ""),
    c("help", "meta", "Describe one command", "command:s@0?"),
    c("capabilities", "meta", "List all commands by group", ""),
    c(
        "auth",
        "meta",
        "Authenticate with the server token",
        "token:s@0",
    ),
    c("shutdown", "meta", "Exit the application", "force:b?"),
    c("state", "query", "App state, camera mode, fps, frame", ""),
    c(
        "scene",
        "query",
        "Loaded map, tiles, entity and asset counts",
        "",
    ),
    c("camera.get", "query", "Camera mode and transform", ""),
    c(
        "sim.get",
        "query",
        "Pause state, simulation time, time scale",
        "",
    ),
    c("log.tail", "query", "Last N log lines", "n:i@0?"),
    c(
        "log.filter",
        "query",
        "Filtered log lines",
        "target:s? level:s? contains:s? n:i?",
    ),
    c(
        "ui.tree",
        "ui",
        "The UI as a tree: id, role, label, state, rect, focusable",
        "visible_only:b?",
    ),
    c("ui.click", "ui", "Activate an element by id", "id:s@0"),
    c(
        "ui.focus",
        "ui",
        "Move keyboard focus to an element",
        "id:s@0",
    ),
    c(
        "ui.action",
        "ui",
        "Send a UI action (navigate right, confirm, back, ...)",
        "action:s@0 arg:s@1?",
    ),
    c("ui.state", "ui", "State of one element by id", "id:s@0"),
    c("entity.list", "query", "List entities", "filter:s@0?"),
    c("entity.get", "query", "Components of one entity", "id:i@0"),
    c("resource.list", "query", "List resource types", ""),
    c("resource.get", "query", "Value of one resource", "name:s@0"),
    c("asset.list", "query", "List loaded assets", "kind:s@0?"),
    c("asset.get", "query", "Metadata of one asset", "path:s@0"),
    c(
        "input.key",
        "input",
        "Press and release a key",
        "key:s@0 duration_ms:i?=duration",
    ),
    c(
        "input.key_hold",
        "input",
        "Press a key and keep it down",
        "key:s@0",
    ),
    c("input.key_release", "input", "Release a key", "key:s@0"),
    c(
        "input.text",
        "input",
        "Type text into the focused field",
        "text:s@0",
    ),
    c(
        "input.mouse.move",
        "input",
        "Move the mouse relatively",
        "dx:f@0 dy:f@1",
    ),
    c(
        "input.mouse.click",
        "input",
        "Click a mouse button",
        "button:s@0 x:f? y:f?",
    ),
    c(
        "input.mouse.scroll",
        "input",
        "Scroll the wheel",
        "delta:f@0",
    ),
    c("sim.pause", "sim", "Pause the simulation", ""),
    c("sim.resume", "sim", "Resume the simulation", ""),
    c(
        "sim.step",
        "sim",
        "Advance N frames while paused",
        "frames:i@0",
    ),
    c(
        "sim.time_scale",
        "sim",
        "Set the simulation speed",
        "factor:f@0",
    ),
    c(
        "sim.set_time",
        "sim",
        "Set the session clock",
        "seconds:f@0",
    ),
    c(
        "scene.load_map",
        "scene",
        "Start a session on a map",
        "path:s@0 entry:s?",
    ),
    c("scene.unload", "scene", "Return to the main menu", ""),
    c(
        "camera.set_mode",
        "scene",
        "Switch camera mode",
        "mode:s@0 sub:i@1?",
    ),
    c(
        "camera.teleport",
        "scene",
        "Move the camera (OMSI coordinates)",
        "x:f@0 y:f@1 z:f@2 yaw:f? pitch:f?",
    ),
    c(
        "render.look",
        "scene",
        "Get or set the colour look (vivid, natural, bevy)",
        "look:s@0?",
    ),
    c(
        "vehicle.set",
        "scene",
        "Replace the player vehicle",
        "path:s@0",
    ),
    c(
        "entity.spawn",
        "scene",
        "Spawn an asset",
        "asset:s@0 transform:j?",
    ),
    c("entity.despawn", "scene", "Despawn an entity", "id:i@0"),
    c(
        "entity.set_component",
        "scene",
        "Set a component value",
        "id:i@0 type:s@1 value:j@2",
    ),
    c(
        "reload.asset",
        "reload",
        "Force a reload of one file",
        "path:s@0",
    ),
    c(
        "reload.scene",
        "reload",
        "Reload every resident tile and the vehicle",
        "",
    ),
    c(
        "reload.config",
        "reload",
        "Reload one configuration file",
        "path:s@0",
    ),
    c("watch.add", "reload", "Watch another path", "path:s@0"),
    c("watch.remove", "reload", "Stop watching a path", "path:s@0"),
    c("watch.list", "reload", "List watched paths", ""),
    c(
        "debug.wireframe",
        "debug",
        "Toggle wireframe rendering",
        "enabled:b@0",
    ),
    c(
        "debug.normals",
        "debug",
        "Toggle normal visualisation",
        "enabled:b@0",
    ),
    c("debug.stats", "debug", "Render and loader statistics", ""),
    c("screenshot", "output", "Save a screenshot", "path:s@0?"),
    c(
        "report",
        "output",
        "File a report (screenshot + text)",
        "text:s@0",
    ),
];

/// A spec for methods this client does not know (a newer server): any `name=value` or `--name value`
/// pair becomes a string/number/bool/JSON parameter.
#[must_use]
pub fn generic_spec() -> CommandSpec {
    c("", "", "", "")
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
        Kind::Json => serde_json::from_str(raw).map_err(|_| bad("JSON")),
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
mod tests {
    use super::*;

    fn params(method: &str, words: &[&str]) -> Result<Value, String> {
        let words: Vec<String> = words.iter().map(|s| (*s).to_string()).collect();
        build_params(find(method).expect("known method"), &words)
    }

    #[test]
    fn positional_and_flag_forms_agree() {
        let a = params("input.key", &["W", "--duration", "2000"]).unwrap();
        let b = params("input.key", &["key=W", "duration_ms=2000"]).unwrap();
        assert_eq!(a, json!({"key": "W", "duration_ms": 2000}));
        assert_eq!(a, b);
    }

    #[test]
    fn typed_values_are_coerced() {
        assert_eq!(params("sim.step", &["60"]).unwrap(), json!({"frames": 60}));
        assert_eq!(
            params("camera.teleport", &["1.5", "2", "3", "--yaw", "90"]).unwrap(),
            json!({"x": 1.5, "y": 2.0, "z": 3.0, "yaw": 90.0})
        );
        assert_eq!(
            params("debug.wireframe", &["on"]).unwrap(),
            json!({"enabled": true})
        );
        assert_eq!(
            params("shutdown", &["--force"]).unwrap(),
            json!({"force": true})
        );
    }

    #[test]
    fn json_flag_supplies_a_whole_object() {
        let v = params(
            "entity.spawn",
            &["--json", r#"{"asset":"a.sco","transform":{"x":1}}"#],
        );
        assert_eq!(v.unwrap(), json!({"asset": "a.sco", "transform": {"x": 1}}));
    }

    #[test]
    fn errors_are_readable() {
        assert!(params("sim.step", &[]).unwrap_err().contains("frames"));
        assert!(params("sim.step", &["many"])
            .unwrap_err()
            .contains("integer"));
        assert!(params("ping", &["x"]).unwrap_err().contains("no value"));
        assert!(params("input.key", &["W", "--nope", "1"])
            .unwrap_err()
            .contains("nope"));
    }

    #[test]
    fn parameterless_commands_send_no_params() {
        assert_eq!(params("ping", &[]).unwrap(), Value::Null);
    }

    #[test]
    fn table_has_unique_names() {
        let mut names: Vec<_> = COMMANDS.iter().map(|c| c.method).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(n, names.len());
    }
}
