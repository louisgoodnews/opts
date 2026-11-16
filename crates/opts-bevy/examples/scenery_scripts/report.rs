//! The printed tables of the probe: the per-script table and the totals.

use super::*;

pub(super) fn print_table(per: &HashMap<String, PerScript>, top: usize) {
    let mut rows: Vec<_> = per.iter().collect();
    rows.sort_by(|a, b| b.1.objects.cmp(&a.1.objects).then(a.0.cmp(b.0)));
    println!(
        "{:<36} {:>7} {:>8} {:>8} {:>8} {:>6} {:>9}",
        "script", "objects", "compile", "missing", "runtime", "live", "us/tick"
    );
    for (name, p) in rows.iter().take(top) {
        println!(
            "{:<36} {:>7} {:>8} {:>8} {:>8} {:>6} {:>9.2}",
            name,
            p.objects,
            p.compile_errors,
            p.missing_files,
            p.runtime_diags,
            p.live,
            p.nanos as f64 / p.ticks.max(1) as f64 / 1000.0
        );
    }
}

/// Prints the totals; `counts` is (.sco files, unparsable, with scripts).
pub(super) fn print_totals(
    per: &HashMap<String, PerScript>,
    messages: HashMap<String, usize>,
    counts: (usize, usize, usize),
) {
    let (files, unparsable, scripted) = counts;
    let sum = |f: fn(&PerScript) -> usize| per.values().map(f).sum::<usize>();
    let objects_with = |f: fn(&PerScript) -> usize| per.values().filter(|p| f(p) > 0).count();
    let ticks: u64 = per.values().map(|p| p.ticks).sum();
    let nanos: u128 = per.values().map(|p| p.nanos).sum();
    println!(
        "\n.sco files: {} ({} unparsable), with scripts: {scripted}, distinct first scripts: {}",
        files,
        unparsable,
        per.len()
    );
    println!(
        "compile errors: {} (in {} script names); missing files: {} (in {}); run-time diagnostics: {} (in {}); live objects: {}",
        sum(|p| p.compile_errors),
        objects_with(|p| p.compile_errors),
        sum(|p| p.missing_files),
        objects_with(|p| p.missing_files),
        sum(|p| p.runtime_diags),
        objects_with(|p| p.runtime_diags),
        sum(|p| p.live)
    );
    let mut top_messages: Vec<_> = messages.into_iter().collect();
    top_messages.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("most common run-time diagnostics:");
    for (m, n) in top_messages.iter().take(15) {
        println!("  {n:>5}  {m}");
    }
    println!(
        "mean cost per tick (host feed + {{frame}}): {:.2} us over {ticks} ticks",
        nanos as f64 / ticks.max(1) as f64 / 1000.0
    );
}
