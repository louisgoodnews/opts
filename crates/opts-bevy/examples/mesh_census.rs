//! Parses every mesh path read from stdin with the engine's own loader entry point and prints a
//! histogram of failures (message with numbers/paths normalised) plus layout classes.
//! Usage: `find <omsi> -iname '*.o3d' -o -iname '*.x' | cargo run -p opts-bevy --example mesh_census`.

use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::Path;

fn normalise(msg: &str) -> String {
    let mut out = String::new();
    let mut last_digit = false;
    for c in msg.chars() {
        if c.is_ascii_digit() {
            if !last_digit {
                out.push('N');
            }
            last_digit = true;
        } else {
            out.push(c);
            last_digit = false;
        }
    }
    out.chars().take(110).collect()
}

fn main() {
    let mut ok = 0usize;
    let mut classes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let path = Path::new(&line);
        let Ok(bytes) = std::fs::read(path) else {
            classes
                .entry("unreadable file".into())
                .or_default()
                .push(line.clone());
            continue;
        };
        match opts_bevy::mesh::parse_mesh_file(path, &bytes) {
            Ok(models) => {
                ok += 1;
                if let Some(m) = models.first() {
                    let k = format!("ok layout={:?}", m.vertex_layout());
                    classes.entry(k).or_default().push(line.clone());
                }
            }
            Err(e) => {
                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                classes
                    .entry(format!("{ext}: {}", normalise(&e)))
                    .or_default()
                    .push(line.clone());
            }
        }
    }
    println!("parsed ok: {ok}");
    let mut rows: Vec<_> = classes.iter().collect();
    rows.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));
    for (k, v) in rows {
        println!("{:6}  {k}\n          e.g. {}", v.len(), v[0]);
        if std::env::var_os("CENSUS_ALL").is_some() && !k.starts_with("ok") {
            for f in v {
                println!("          - {f}");
            }
        }
    }
}
