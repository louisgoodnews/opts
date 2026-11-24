//! Line helpers of the sound configuration parser: numbers, keywords and parameter lines.

pub(super) fn num(s: &str) -> Option<f32> {
    s.trim().replace(',', ".").parse::<f32>().ok()
}

pub(super) fn keyword(line: &str) -> Option<String> {
    let t = line.trim();
    (t.starts_with('[') && t.ends_with(']') && t.len() > 2).then(|| t.to_lowercase())
}

pub(super) fn params<'a>(lines: &[&'a str], at: usize, n: usize) -> (Vec<&'a str>, usize) {
    let mut got = Vec::new();
    let mut j = at + 1;
    while j < lines.len() && got.len() < n {
        let l = lines[j];
        if l.is_empty() {
            j += 1;
            continue;
        }
        if keyword(l).is_some() {
            break;
        }
        got.push(l);
        j += 1;
    }
    (got, j)
}
