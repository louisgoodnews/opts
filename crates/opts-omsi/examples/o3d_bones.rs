//! Prints the bone chunk of the `.o3d` files read from stdin (names, influenced vertices).
use std::io::BufRead;

use opts_omsi::formats::o3d::parse;

fn main() {
    for p in std::io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(b) = std::fs::read(&p) else { continue };
        let Ok(o) = parse(&b) else {
            println!("{p}: parse error");
            continue;
        };
        println!("  key {:?} verts {}", o.encryption_key, o.vertices.len());
        match o.bone_influences() {
            Some(bones) => println!(
                "{p}: {} bones {:?}",
                bones.len(),
                bones
                    .iter()
                    .map(|b| (b.name.as_str(), b.entries.len()))
                    .collect::<Vec<_>>()
            ),
            None => println!("{p}: no bone chunk (version {})", o.version),
        }
    }
}
