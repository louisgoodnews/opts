//! Discovery and loading of `.ron` theme files from a folder.

use std::path::PathBuf;

use opts_theme_files::{discover_in, load_from};

fn dir() -> PathBuf {
    // Integration tests write under the test target folder, never the system temp folder.
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("opts-theme-files-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

#[test]
fn files_are_discovered_loaded_and_bad_ones_ignored() {
    let d = dir();
    std::fs::write(d.join("good.ron"), r#"(version: 1, name: "Good")"#).expect("write");
    std::fs::write(d.join("bad.ron"), "nonsense").expect("write");
    std::fs::write(d.join("note.txt"), "x").expect("write");
    assert_eq!(discover_in(&d), ["bad", "good"]);
    assert!(load_from(&d, "good").is_some());
    assert!(load_from(&d, "bad").is_none());
    assert!(load_from(&d, "missing").is_none());
    assert!(load_from(&d, "../good").is_none(), "no path escapes");
    assert!(load_from(&d, "").is_none());
    let _ = std::fs::remove_dir_all(&d);
}
