//! Scans an OMSI installation into a catalogue database and prints a summary.
//! Usage: `cargo run --release -p opts-catalogue --example scan_omsi -- <OMSI root> <db path>`

use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("OMSI root"));
    let db = PathBuf::from(args.next().expect("db path"));
    let t = std::time::Instant::now();
    let summary = opts_catalogue::discover(&db, &root, &|p| {
        eprintln!("phase {p:?} at {:?}", t.elapsed())
    })
    .expect("scan");
    println!("{summary:?} in {:?}", t.elapsed());
    let cat = opts_catalogue::Catalogue::open(&db).expect("open");
    for (c, n) in cat.category_counts().expect("counts") {
        println!("{c:15} {n}");
    }
    println!(
        "maps {} | playable vehicles {}",
        cat.maps().expect("maps").len(),
        cat.playable_vehicles().expect("veh").len()
    );
}
