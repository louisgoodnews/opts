//! Finding the `.bus` files of the corpus and the script files a `.bus` names.

use std::path::{Path, PathBuf};

use opts_sim::script::FileRead;

/// Reads a script file that the lookup found: the loader only needs the outcome.
pub(crate) fn read_script(path: Option<PathBuf>) -> FileRead {
    let Some(path) = path else {
        return FileRead::NotFound;
    };
    match std::fs::read(&path) {
        Ok(bytes) => FileRead::Loaded {
            name: path.to_string_lossy().into_owned(),
            bytes,
        },
        Err(e) => FileRead::Unreadable(e.to_string()),
    }
}

pub(crate) fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for comp in rel
        .split(['\\', '/'])
        .filter(|c| !c.is_empty() && *c != ".")
    {
        let direct = cur.join(comp);
        if direct.exists() {
            cur = direct;
            continue;
        }
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(comp))?;
        cur = hit.path();
    }
    Some(cur)
}

pub(crate) fn buses(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            buses(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus")) {
            out.push(p);
        }
    }
}
