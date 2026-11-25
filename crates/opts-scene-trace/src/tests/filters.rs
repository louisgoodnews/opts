//! Record filters: tile, kind, file glob and radius.

use super::*;

#[test]
fn filters_by_tile_kind_file_and_radius() {
    let _g = guard();
    set_directory(scratch("filters"));
    set_mode(Mode::Full);
    begin_session("Testmap", Path::new("/omsi"));
    set_filters(Filters {
        tiles: vec![(2, 2)],
        ..Filters::default()
    });
    assert!(put(kind::SCENERY, (1, 1), "a.sco", [0.0; 3]).is_none());
    assert!(put(kind::SCENERY, (2, 2), "a.sco", [0.0; 3]).is_some());

    set_filters(Filters {
        kinds: vec![kind::TREE.into()],
        ..Filters::default()
    });
    assert!(put(kind::SCENERY, (1, 1), "a.sco", [0.0; 3]).is_none());
    assert!(put(kind::TREE, (1, 1), "a.sco", [0.0; 3]).is_some());

    set_filters(Filters {
        file: Some("Haus*.SCO".into()),
        ..Filters::default()
    });
    assert!(put(
        kind::SCENERY,
        (1, 1),
        "Sceneryobjects/x/haus_01.sco",
        [0.0; 3]
    )
    .is_some());
    assert!(put(kind::SCENERY, (1, 1), "Sceneryobjects/x/baum.sco", [0.0; 3]).is_none());

    set_filters(Filters {
        radius: Some(Radius {
            center: [100.0, 100.0],
            meters: 10.0,
        }),
        ..Filters::default()
    });
    assert!(put(kind::SCENERY, (1, 1), "a.sco", [105.0, 0.0, 100.0]).is_some());
    assert!(put(kind::SCENERY, (1, 1), "a.sco", [0.0, 0.0, 0.0]).is_none());
    set_mode(Mode::Off);
}
