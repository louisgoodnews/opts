//! Does `MapTile::parse` keep every record of a map's tiles? Counts the raw section headers of
//! every `tile_*.map` (`[spline]`/`[spline_h]`, `[object]`, `[splineAttachement]`/`_repeater`,
//! `[attachObj]`) and compares them with the parsed tile. Prints the tiles where the counts differ and the totals.
//!
//! Usage: `tile_record_census <map dir>`
use std::collections::BTreeMap;

use opts_core::formats::map::MapTile;

fn text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).expect("map dir"));
    let mut totals: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut differing = 0;
    let mut tiles = 0;
    for e in std::fs::read_dir(&dir).expect("dir").flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("tile_") && name.ends_with(".map")) {
            continue;
        }
        tiles += 1;
        let bytes = std::fs::read(e.path()).expect("tile");
        let raw = text(&bytes);
        let count = |keys: &[&str]| {
            raw.lines()
                .filter(|l| keys.iter().any(|k| l.trim().eq_ignore_ascii_case(k)))
                .count()
        };
        let raw_splines = count(&["[spline]", "[spline_h]"]);
        let raw_objects = count(&["[object]"]);
        let raw_attached = count(&["[attachObj]"]);
        let raw_attach = count(&["[splineAttachement]", "[splineAttachement_repeater]"]);
        let parsed = match MapTile::parse(&bytes) {
            Ok(t) => t,
            Err(err) => {
                println!("{name}: parse failed: {err}");
                differing += 1;
                continue;
            }
        };
        let rows = [
            ("splines", raw_splines, parsed.splines.len()),
            ("objects", raw_objects, parsed.objects.len()),
            ("attachments", raw_attach, parsed.spline_attachments.len()),
            ("attachObj", raw_attached, parsed.attached_objects.len()),
        ];
        let mut line = String::new();
        for (kind, r, p) in rows {
            let t = totals.entry(kind).or_default();
            t.0 += r;
            t.1 += p;
            if r != p {
                line.push_str(&format!(" {kind} raw {r} parsed {p};"));
            }
        }
        if !line.is_empty() {
            differing += 1;
            println!("{name}:{line}");
        }
    }
    println!("tiles {tiles}, differing {differing}");
    for (k, (r, p)) in totals {
        println!("  {k:12} raw {r:7} parsed {p:7}");
    }
}
