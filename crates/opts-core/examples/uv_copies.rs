//! How many copies (keyed or plain) of the meshes of one vehicle folder exist elsewhere in the
//! installation (same file name, vertex and triangle counts)? Copies allow twin substitution or joint
//! decoding. Usage: `uv_copies <OMSI root> <vehicle folder>`.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use opts_core::formats::o3d::parse;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("o3d")) {
            out.push(p);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap());
    let folder = PathBuf::from(args.next().unwrap());
    let mut mine = Vec::new();
    walk(&folder, &mut mine);
    let mut all = Vec::new();
    walk(&root, &mut all);
    let key = |p: &Path, bytes: &[u8]| {
        let o = parse(bytes).ok()?;
        Some((p.file_name()?.to_string_lossy().to_lowercase(), o.vertices.len(), o.indices.len(), o.encryption_key.is_some()))
    };
    let mut want: HashMap<(String, usize, usize), (bool, usize, usize)> = HashMap::new(); // keyed?, other keyed copies, plain copies
    for p in &mine {
        if let Some((n, v, f, keyed)) = key(p, &std::fs::read(p).unwrap()) {
            want.insert((n, v, f), (keyed, 0, 0));
        }
    }
    for p in all.iter().filter(|p| !p.starts_with(&folder)) {
        let Ok(b) = std::fs::read(p) else { continue };
        if b.len() < 8 || b[0] != 0x84 || b[1] != 0x19 || b[2] < 4 {
            continue;
        }
        if let Some((n, v, f, keyed)) = key(p, &b) {
            if let Some(e) = want.get_mut(&(n, v, f)) {
                if keyed {
                    e.1 += 1;
                } else {
                    e.2 += 1;
                }
            }
        }
    }
    let keyed_meshes = want.values().filter(|e| e.0).count();
    let with_plain = want.values().filter(|e| e.0 && e.2 > 0).count();
    let with_keyed = want.values().filter(|e| e.0 && e.1 > 0 && e.2 == 0).count();
    println!("{} distinct meshes in the folder, {} keyed; of those {} have a plain copy elsewhere (same name and counts), {} more have only keyed copies elsewhere", want.len(), keyed_meshes, with_plain, with_keyed);
}
