//! Unit tests for `super` (`prune.rs`).

use super::*;

#[test]
fn only_captures_with_a_sidecar_or_manifest_are_candidates() {
    let root = std::env::temp_dir().join(format!("opts_prune_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("burst")).expect("dir");
    std::fs::write(root.join("a.png"), b"x").expect("file");
    std::fs::write(root.join("a.png.json"), b"{}").expect("sidecar");
    std::fs::write(root.join("mine.txt"), b"not a capture").expect("other");
    std::fs::write(root.join("burst").join("manifest.json"), b"{}").expect("manifest");
    std::fs::create_dir_all(root.join("other")).expect("dir");
    std::fs::write(root.join("other").join("keep.png"), b"x").expect("keep");
    let far = SystemTime::now() + Duration::from_secs(3600);
    let found: Vec<String> = find_old(&root, far)
        .iter()
        .map(|i| {
            i.path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(found, ["a.png", "burst"]);
    assert!(
        find_old(&root, SystemTime::UNIX_EPOCH).is_empty(),
        "nothing is old yet"
    );
}
