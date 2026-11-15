//! Session modes: the off switch, full and summary records.

use super::*;

#[test]
fn off_allocates_nothing() {
    let _g = guard();
    set_mode(Mode::Off);
    assert!(!enabled());
    let id = record(
        &Probe {
            kind: kind::SCENERY,
            tile: Some((0, 0)),
            file: "a.sco",
            world: [0.0; 3],
        },
        || panic!("the record must not be built while the trace is off"),
    );
    assert_eq!(id, None);
    note_mesh(Path::new("x.o3d"), MeshNote::default());
    begin_tile((0, 0));
    end_tile((0, 0));
    assert!(!state_allocated(), "no state while off");
    assert!(!Lap::start().active());
    assert_eq!(status().mode, "");
}

#[test]
fn full_mode_keeps_every_record_and_summary_only_issues() {
    let _g = guard();
    set_directory(scratch("modes"));
    set_mode(Mode::Full);
    begin_session("Testmap", Path::new("/omsi"));
    put(kind::SCENERY, (1, 1), "Sceneryobjects/a.sco", [0.0; 3]).unwrap();
    put(kind::TREE, (1, 1), "Sceneryobjects/t.sco", [5.0, 0.0, 0.0]).unwrap();
    assert_eq!(status().in_memory, 2);
    set_mode(Mode::Off);

    set_mode(Mode::Summary);
    begin_session("Testmap", Path::new("/omsi"));
    put(kind::SCENERY, (1, 1), "a.sco", [0.0; 3]).unwrap();
    let (mut bad, w) = rec(kind::SCENERY, (1, 1), "b.sco", [0.0; 3]);
    bad.warnings.push("o3d not found".into());
    record(
        &Probe {
            kind: kind::SCENERY,
            tile: Some((1, 1)),
            file: "b.sco",
            world: w,
        },
        || bad,
    )
    .unwrap();
    let s = status();
    assert_eq!(s.records, 2);
    assert_eq!(
        s.in_memory, 1,
        "summary keeps only the record with an issue"
    );
    let t = tile_summary((1, 1)).unwrap();
    assert_eq!((t.total, t.issue_total), (2, 1));
    set_mode(Mode::Off);
}
