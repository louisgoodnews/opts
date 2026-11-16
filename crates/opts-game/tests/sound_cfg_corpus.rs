//! The scenery sound configurations of the installation parse without surprises (M-0139):
//! every `Sceneryobjects/**/sound/*.cfg` reads, its definitions have a file name, a position
//! and a range, and the keywords seen are the ones the format page lists. Skipped without
//! `OMSI_PATH`.

use std::path::{Path, PathBuf};

use opts_core::formats::sound::SoundConfig;

fn cfgs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            cfgs(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("cfg"))
            // `rw_c_p_concrete.bmp.cfg` and its kind are texture configurations (`[moisture]`,
            // `[puddles]`, `[surface]`), not sound configurations.
            && !p.file_stem().is_some_and(|s| s.to_string_lossy().contains('.'))
            && p.parent()
                .and_then(|d| d.file_name())
                .is_some_and(|n| n.eq_ignore_ascii_case("sound"))
        {
            out.push(p);
        }
    }
}

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

fn vehicle_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            vehicle_files(&p, out);
        } else if p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("bus") || x.eq_ignore_ascii_case("ovh"))
        {
            out.push(p);
        }
    }
}
#[path = "sound_cfg_corpus/ai.rs"]
mod ai;
#[path = "sound_cfg_corpus/player.rs"]
mod player;
#[path = "sound_cfg_corpus/scenery.rs"]
mod scenery;
