//! Plan keys and command-line arguments.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct Cfg {
    pub(super) label: String,
    pub(super) kv: BTreeMap<String, String>,
}

impl Cfg {
    pub(super) fn f(&self, k: &str, d: f32) -> f32 {
        self.kv.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
    }
    pub(super) fn u(&self, k: &str, d: u32) -> u32 {
        self.kv.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
    }
    pub(super) fn s<'a>(&'a self, k: &str, d: &'a str) -> &'a str {
        self.kv.get(k).map_or(d, String::as_str)
    }
}

pub(super) fn parse_kv(s: &str) -> BTreeMap<String, String> {
    s.split(',')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

pub(super) fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1).cloned())
}
