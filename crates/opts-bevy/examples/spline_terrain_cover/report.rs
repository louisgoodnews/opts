//! Printing of the coverage report.

use super::*;

type Tally<'a> = BTreeMap<(&'a str, &'a str), (u64, u64)>;

pub(super) fn print_report(
    n_tiles: usize,
    drawn: usize,
    counts: (&Tally, &BTreeMap<&str, usize>),
    worst: &mut [(u64, String, (i32, i32), u64)],
) {
    let (tally, bad_splines) = counts;
    println!("tiles {n_tiles}, drawn splines {drawn}");
    for ((fp, own), (n, c)) in tally {
        println!(
            "{fp:13} {own:15} sampled {n:8} m  covered {c:7} m ({:.2} %)",
            100.0 * *c as f64 / (*n).max(1) as f64
        );
    }
    println!("splines with >= 2 m covered: {bad_splines:?}");
    worst.sort_by_key(|w| std::cmp::Reverse(w.0));
    for (c, sli, key, id) in worst.iter().take(15) {
        println!("  {c:5} m  tile {key:?} id {id}  {sli}");
    }
}
