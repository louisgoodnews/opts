//! Tests of the hot reload events: the extensions, the editor scratch files, and the bounded history.

use super::*;

#[test]
fn extensions_map_to_kinds() {
    let k = |p: &str| classify(Path::new(p));
    assert_eq!(k("a/b.O3D"), Some(ReloadKind::Mesh));
    assert_eq!(k("a.x"), Some(ReloadKind::Mesh));
    assert_eq!(k("t.dds"), Some(ReloadKind::Texture));
    assert_eq!(k("tile_1_2.map"), Some(ReloadKind::Tile));
    assert_eq!(k("tile_1_2.map.terrain"), Some(ReloadKind::Terrain));
    assert_eq!(k("m.cfg"), Some(ReloadKind::Config));
    assert_eq!(k("notes.txt"), None);
    assert_eq!(k("noext"), None);
}

#[test]
fn editor_scratch_files_are_recognised() {
    for name in [
        "a.sco~",
        ".a.sco.swp",
        ".#a.sco",
        "#a.sco#",
        "a.sco.tmp",
        "a.o3d.bak",
    ] {
        assert!(is_editor_scratch(Path::new(name)), "{name}");
    }
    assert!(!is_editor_scratch(Path::new("model_main.cfg")));
}

#[test]
fn stats_keep_a_bounded_history() {
    let mut s = ReloadStats::default();
    for i in 0..100 {
        let seq = s.allocate();
        s.record(ReloadOutcome {
            seq,
            path: PathBuf::from(format!("{i}")),
            result: Ok(String::new()),
        });
    }
    assert_eq!(s.outcomes.len(), ReloadStats::KEEP);
    assert!(s.outcome(100).is_some());
    assert!(s.outcome(1).is_none());
    assert_eq!(s.applied, 100);
}
