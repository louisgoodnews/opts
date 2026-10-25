//! Finding OMSI 2 through Steam's library configuration
//! (`docs/src/architecture/application-startup.md`, "Finding OMSI through Steam").
//!
//! Steam lists its libraries in `steamapps/libraryfolders.vdf` and names a game's folder in
//! `appmanifest_<appid>.acf`, both in Valve's KeyValues text format. OMSI 2 is app 252530. Only
//! reads; a missing or unreadable file is skipped.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::omsi_root::is_omsi_root;

/// The Steam app id of OMSI 2.
pub const OMSI2_APP_ID: &str = "252530";

/// The folder Steam uses for OMSI 2 when its manifest is missing.
const DEFAULT_INSTALL_DIR: &str = "OMSI 2";

/// A KeyValues value: a string or a block of key-value pairs (keys keep their order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kv {
    /// A quoted or bare string.
    Str(String),
    /// A `{ … }` block.
    Block(Vec<(String, Kv)>),
}

impl Kv {
    /// The value of `key` in a block (keys compare without case, as Steam does).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Kv> {
        match self {
            Kv::Block(pairs) => pairs
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Kv::Str(_) => None,
        }
    }

    /// The string, if this is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Kv::Str(s) => Some(s),
            Kv::Block(_) => None,
        }
    }
}

/// Splits KeyValues text into tokens: strings (quoted with `\\`, `\"`, `\n`, `\t` escapes, or
/// bare), `{` and `}`. `//` comments and `[$WIN32]`-style conditions are skipped.
fn tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => out.push(Token::Open),
            '}' => out.push(Token::Close),
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(other) => s.push(other),
                            None => break,
                        },
                        _ => s.push(c),
                    }
                }
                out.push(Token::Str(s));
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '[' => {
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            c => {
                let mut s = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() || matches!(n, '"' | '{' | '}') {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                out.push(Token::Str(s));
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Str(String),
    Open,
    Close,
}

/// Parses KeyValues text into the top-level block. Malformed input yields what was read up to
/// the error (Steam's files are machine-written; a truncated file still gives its first entries).
#[must_use]
pub fn parse(text: &str) -> Kv {
    let tokens = tokens(text);
    let mut pos = 0;
    Kv::Block(block(&tokens, &mut pos))
}

fn block(tokens: &[Token], pos: &mut usize) -> Vec<(String, Kv)> {
    let mut pairs = Vec::new();
    while let Some(t) = tokens.get(*pos) {
        *pos += 1;
        let key = match t {
            Token::Str(k) => k.clone(),
            Token::Close => break,
            Token::Open => continue,
        };
        match tokens.get(*pos) {
            Some(Token::Str(v)) => {
                *pos += 1;
                pairs.push((key, Kv::Str(v.clone())));
            }
            Some(Token::Open) => {
                *pos += 1;
                pairs.push((key, Kv::Block(block(tokens, pos))));
            }
            Some(Token::Close) | None => break,
        }
    }
    pairs
}

/// One Steam library: its folder and whether its `apps` list names OMSI 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    /// The library folder (it holds `steamapps/`).
    pub path: PathBuf,
    /// Whether Steam lists OMSI 2 in it (only known in the current file format).
    pub has_omsi: bool,
}

/// The libraries of a `libraryfolders.vdf`, in both the current format
/// (`"0" { "path" "…" "apps" { … } }`) and the one before 2021 (`"1" "D:\\SteamLibrary"`).
#[must_use]
pub fn libraries(vdf: &str) -> Vec<Library> {
    let root = parse(vdf);
    let Some(Kv::Block(entries)) = root.get("libraryfolders") else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|(key, _)| key.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|(_, value)| match value {
            Kv::Str(path) => Some(Library {
                path: PathBuf::from(path),
                has_omsi: false,
            }),
            Kv::Block(_) => Some(Library {
                path: PathBuf::from(value.get("path")?.as_str()?),
                has_omsi: value
                    .get("apps")
                    .is_some_and(|apps| apps.get(OMSI2_APP_ID).is_some()),
            }),
        })
        .collect()
}

/// The `installdir` of an `appmanifest_<appid>.acf`.
#[must_use]
pub fn install_dir(acf: &str) -> Option<String> {
    parse(acf)
        .get("AppState")?
        .get("installdir")?
        .as_str()
        .map(str::to_owned)
        .filter(|d| !d.trim().is_empty())
}

/// The OMSI 2 installation known to the Steam installations `steam_roots`, if any: the
/// libraries that list OMSI 2 first, then the others (each installation's own folder included).
#[must_use]
pub fn find_omsi(steam_roots: &[PathBuf]) -> Option<PathBuf> {
    let mut listed = Vec::new();
    let mut others = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |path: PathBuf, has_omsi: bool| {
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if seen.insert(key) {
            if has_omsi {
                listed.push(path);
            } else {
                others.push(path);
            }
        }
    };
    for steam in steam_roots.iter().filter(|p| p.is_dir()) {
        let found: Vec<Library> = ["steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"]
            .iter()
            .filter_map(|f| std::fs::read_to_string(steam.join(f)).ok())
            .flat_map(|text| libraries(&text))
            .collect();
        add(
            steam.clone(),
            found.iter().any(|l| l.has_omsi && same(&l.path, steam)),
        );
        for lib in found {
            add(lib.path, lib.has_omsi);
        }
    }
    listed
        .into_iter()
        .chain(others)
        .find_map(|library| omsi_in_library(&library))
}

/// Whether two paths name the same folder (symbolic links resolved where possible).
fn same(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(a) == canon(b)
}

/// OMSI 2 in one library: the folder its manifest names, else the default one.
fn omsi_in_library(library: &Path) -> Option<PathBuf> {
    let steamapps = library.join("steamapps");
    let dir = std::fs::read_to_string(steamapps.join(format!("appmanifest_{OMSI2_APP_ID}.acf")))
        .ok()
        .and_then(|acf| install_dir(&acf))
        .unwrap_or_else(|| DEFAULT_INSTALL_DIR.to_owned());
    let root = steamapps.join("common").join(dir);
    is_omsi_root(&root).then_some(root)
}

/// The Steam installations of this platform, most likely first (they may not exist).
#[must_use]
pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if cfg!(windows) {
        roots.extend(windows_registry_roots());
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(dir) = std::env::var_os(var) {
                roots.push(PathBuf::from(dir).join("Steam"));
            }
        }
    } else if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        let relative: &[&str] = if cfg!(target_os = "macos") {
            &["Library/Application Support/Steam"]
        } else {
            &[
                ".steam/steam",
                ".steam/root",
                ".local/share/Steam",
                ".var/app/com.valvesoftware.Steam/.local/share/Steam",
                ".var/app/com.valvesoftware.Steam/data/Steam",
                "snap/steam/common/.local/share/Steam",
            ]
        };
        roots.extend(relative.iter().map(|r| home.join(r)));
    }
    roots
}

/// Steam's folder from the Windows registry (`reg query`, part of every Windows).
fn windows_registry_roots() -> Vec<PathBuf> {
    [
        (r"HKCU\Software\Valve\Steam", "SteamPath"),
        (r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
        (r"HKLM\SOFTWARE\Valve\Steam", "InstallPath"),
    ]
    .iter()
    .filter_map(|(key, value)| {
        let out = std::process::Command::new("reg")
            .args(["query", key, "/v", value])
            .output()
            .ok()?;
        out.status.success().then_some(())?;
        registry_value(&String::from_utf8_lossy(&out.stdout), value).map(PathBuf::from)
    })
    .collect()
}

/// The data of `value` in the output of `reg query <key> /v <value>`
/// (`    SteamPath    REG_SZ    c:/program files (x86)/steam`).
fn registry_value(output: &str, value: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(value)?;
        let rest = rest.trim_start();
        let (kind, data) = rest.split_once(char::is_whitespace)?;
        kind.starts_with("REG_")
            .then(|| data.trim().to_owned())
            .filter(|d| !d.is_empty())
    })
}

#[cfg(test)]
mod tests;
