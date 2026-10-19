//! The relocation search: prefix, score and candidate choice.

/// Number of leading path components shared by two `/`-separated paths (case-insensitive).
fn shared_prefix(a: &str, b: &str) -> usize {
    a.split('/')
        .zip(b.split('/'))
        .take_while(|(x, y)| x.eq_ignore_ascii_case(y))
        .count()
}

/// Picks the best of several same-named files for a reference made from `near_dir`:
/// the one sharing the longest leading path, then the shortest path.
pub(super) fn best_candidate<'a>(candidates: &'a [String], near_dir: &str) -> Option<&'a String> {
    candidates.iter().max_by(|a, b| {
        shared_prefix(a, near_dir)
            .cmp(&shared_prefix(b, near_dir))
            .then_with(|| b.len().cmp(&a.len()))
    })
}

/// Chooses among same-named files for a reference made from `ref_dir`: a single candidate is
/// taken; of several, the best folder match when it beats the runner-up clearly.
pub(super) fn pick_relocated<'a>(found: &'a [String], ref_dir: &str) -> Option<&'a String> {
    match found {
        [] => None,
        [one] => Some(one),
        _ => {
            let mut scored: Vec<((usize, usize), &String)> = found
                .iter()
                .map(|c| (relocation_score(c, ref_dir), c))
                .collect();
            scored.sort_by_key(|s| std::cmp::Reverse(s.0));
            let (best, second) = (scored[0].0, scored[1].0);
            (best > second && best != (0, 0)).then_some(scored[0].1)
        }
    }
}

/// How well the folders of candidate `cand` match the folders of a reference made from `ref_dir`:
/// `(equal trailing folders, shared name tokens)`.
fn relocation_score(cand: &str, ref_dir: &str) -> (usize, usize) {
    let cand_dir = cand.rsplit_once('/').map_or("", |(d, _)| d);
    let trailing = cand_dir
        .rsplit('/')
        .zip(ref_dir.rsplit('/'))
        .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
        .count();
    const GENERIC: [&str; 6] = [
        "splines",
        "sceneryobjects",
        "vehicles",
        "texture",
        "model",
        "maps",
    ];
    let tokens = |d: &str| -> Vec<String> {
        d.split(|c: char| !c.is_alphanumeric())
            .map(str::to_lowercase)
            .filter(|t| t.len() >= 3 && !GENERIC.contains(&t.as_str()))
            .collect()
    };
    let (a, b) = (tokens(cand_dir), tokens(ref_dir));
    (trailing, a.iter().filter(|t| b.contains(t)).count())
}

pub(super) fn normalize(rel: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in rel.split(['\\', '/']) {
        match c.trim() {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}
