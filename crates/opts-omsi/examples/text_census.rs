//! Parses `.sco`, `.sli`, `.bus` and `model*.cfg` paths from stdin and prints failure histograms.
use std::collections::BTreeMap;
use std::io::BufRead;

use opts_omsi::formats::scenery::{sco::SceneryObject, sli::SplineProfile};
use opts_omsi::formats::vehicles::{bus::BusManifest, model::ModelConfig};

fn norm(s: &str) -> String {
    let mut out = String::new();
    let mut digit = false;
    for c in s.chars().take(120) {
        if c.is_ascii_digit() {
            if !digit {
                out.push('N');
            }
            digit = true;
        } else {
            out.push(c);
            digit = false;
        }
    }
    out
}

fn main() {
    let mut ok: BTreeMap<String, u32> = BTreeMap::new();
    let mut fails: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let ext = line.rsplit('.').next().unwrap_or("").to_lowercase();
        let Ok(b) = std::fs::read(&line) else {
            continue;
        };
        let res: Result<(), String> = match ext.as_str() {
            "sco" => SceneryObject::parse(&b)
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "sli" => SplineProfile::parse(&b)
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "bus" => BusManifest::parse_bytes(&b)
                .map(|_| ())
                .map_err(|e| e.to_string()),
            "cfg" => {
                let name = line.rsplit('/').next().unwrap_or("").to_lowercase();
                if !name.starts_with("model") {
                    continue;
                }
                ModelConfig::parse_bytes(&b)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            _ => continue,
        };
        match res {
            Ok(()) => *ok.entry(ext).or_default() += 1,
            Err(e) => fails.entry((ext, norm(&e))).or_default().push(line),
        }
    }
    println!("ok: {ok:?}");
    let mut rows: Vec<_> = fails.iter().collect();
    rows.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
    for ((ext, msg), v) in rows {
        println!("{:5} {ext}: {msg}\n        e.g. {}", v.len(), v[0]);
    }
}
