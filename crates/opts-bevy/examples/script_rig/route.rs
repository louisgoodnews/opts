//! The route report and its helpers.

use super::phases::Loop;
use super::*;

mod report;
pub(super) use report::route_report;

/// System values the engine itself sets in every `run_frame` (`GetTime`, `Timegap`): never unfed.
pub(super) const ENGINE_SET: &[&str] = &["gettime", "timegap"];

pub(super) fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for comp in rel
        .split(['\\', '/'])
        .filter(|c| !c.is_empty() && *c != ".")
    {
        if comp == ".." {
            cur = cur.parent()?.to_path_buf();
            continue;
        }
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

pub(super) fn buses(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            buses(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus")) {
            out.push(p);
        }
    }
}

/// A stable seed from the vehicle's path (FNV-1a).
pub(super) fn seed_of(path: &str) -> u64 {
    path.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
    })
}

/// The output rows the scripts wrote that OPTS does not read yet.
pub(super) fn unconsumed(writes: &BTreeSet<String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for w in writes {
        for row in OUTPUT_TABLE {
            let hit = if row.name.ends_with('_') {
                w.starts_with(row.name)
            } else {
                w == row.name
            };
            if hit && !row.consumed {
                out.insert(row.name.to_owned());
            }
        }
    }
    out
}

/// The depot of a vehicle: a `.hof` of its folder whose name contains `filter`, else the game's
/// rule ([`opts_bevy::route::find_depot`]).
pub(super) fn depot_for(
    dir: &Path,
    root: &Path,
    filter: Option<&str>,
) -> Option<std::sync::Arc<opts_core::formats::hof::HofFile>> {
    let pick = filter.and_then(|f| {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|e| e.eq_ignore_ascii_case("hof"))
                    && p.file_name()
                        .is_some_and(|n| n.to_string_lossy().to_lowercase().contains(f))
            })
            .collect();
        files.sort();
        files.iter().find_map(|p| {
            opts_core::formats::hof::HofFile::parse_bytes(&std::fs::read(p).ok()?).ok()
        })
    });
    pick.or_else(|| opts_bevy::route::find_depot(dir, root, &[]))
        .map(std::sync::Arc::new)
}

type Depot = Option<std::sync::Arc<opts_core::formats::hof::HofFile>>;
type Timetable<'a> = Option<(&'a (String, usize, usize), PathBuf)>;

/// The names of the held string variables that carry one of the wanted strings.
fn carrying(held: &[(String, String)], wanted: &[String]) -> Vec<String> {
    held.iter()
        .filter(|(_, v)| {
            // Displays pad and spread their text: compare without spaces, by containment.
            let flat = |t: &str| t.chars().filter(|c| !c.is_whitespace()).collect::<String>();
            let have = flat(v);
            wanted.iter().any(|w| {
                let w = flat(w);
                w.chars().count() >= 3 && have.contains(&w)
            })
        })
        .map(|(n, _)| n.clone())
        .collect()
}

/// The variables named in `RIG_VARS` (comma separated) with their values.
fn extra_vars(lp: &Loop) -> serde_json::Map<String, serde_json::Value> {
    std::env::var("RIG_VARS")
        .map(|l| {
            l.split(',')
                .map(|n| {
                    (
                        n.to_string(),
                        serde_json::json!(lp.engine.get(&n.to_lowercase())),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}
