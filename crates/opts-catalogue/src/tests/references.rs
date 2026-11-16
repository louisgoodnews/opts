//! Reference-rewriting and extraction tests of the catalogue.

use super::*;

#[test]
fn reroutes_references_to_same_named_files_elsewhere() {
    let root = fixture();
    fs::create_dir_all(root.join("Sceneryobjects/PackA/model")).unwrap();
    fs::create_dir_all(root.join("Sceneryobjects/PackB/texture")).unwrap();
    fs::write(
        root.join("Sceneryobjects/PackA/a.sco"),
        "[mesh]\na.o3d\n[matl]\nlost.bmp\n0\n[matl]\ngone.bmp\n1\n",
    )
    .unwrap();
    fs::write(root.join("Sceneryobjects/PackA/model/a.o3d"), b"x").unwrap();
    fs::write(root.join("Sceneryobjects/PackB/texture/LOST.dds"), b"x").unwrap(); // other pack, other extension
    let db = root.join("cat.db");
    let summary = discover(&db, &root, &|_| {}).unwrap();
    assert_eq!(summary.rerouted, 1, "{summary:?}");
    assert_eq!(summary.unresolved, 1, "gone.bmp exists nowhere");
    let cat = Catalogue::open(&db).unwrap();
    let state: i64 = cat
        .with(|c| {
            c.query_row(
                "SELECT is_resolved FROM entity_references WHERE raw_reference='lost.bmp'",
                [],
                |r| r.get(0),
            )
        })
        .unwrap();
    assert_eq!(state, 2);
    let found = cat
        .find_by_name("textures\\lost.bmp", "Sceneryobjects/PackA", true)
        .unwrap();
    assert!(found.is_some_and(|p| p.ends_with("PackB/texture/LOST.dds")));
    assert!(cat.find_by_name("gone.bmp", "", true).unwrap().is_none());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn extracts_scenery_and_spline_references() {
    let root = fixture();
    fs::create_dir_all(root.join("Sceneryobjects/Pack/model")).unwrap();
    fs::create_dir_all(root.join("Sceneryobjects/Pack/texture")).unwrap();
    fs::create_dir_all(root.join("Splines/Road/texture")).unwrap();
    fs::create_dir_all(root.join("Texture")).unwrap();
    fs::write(
        root.join("Sceneryobjects/Pack/house.sco"),
        "[friendlyname]\nHouse\n[mesh]\nhouse.o3d\n[matl]\nwall.bmp\n0\n[matl]\nmissing.bmp\n1\n",
    )
    .unwrap();
    fs::write(root.join("Sceneryobjects/Pack/model/house.o3d"), b"x").unwrap();
    fs::write(root.join("Sceneryobjects/Pack/texture/wall.dds"), b"x").unwrap(); // other extension
    fs::write(
        root.join("Splines/Road/street.sli"),
        "[texture]\nasphalt.bmp\n",
    )
    .unwrap();
    fs::write(root.join("Texture/asphalt.bmp"), b"x").unwrap(); // global fallback
    let db = root.join("cat.db");
    discover(&db, &root, &|_| {}).unwrap();
    let cat = Catalogue::open(&db).unwrap();
    let rows: Vec<(String, String, bool)> = cat
        .with(|c| {
            let mut st = c.prepare(
                "SELECT role, raw_reference, is_resolved FROM entity_references ORDER BY role, raw_reference",
            )?;
            let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? == 1)))?;
            r.collect()
        })
        .unwrap();
    let has = |role: &str, raw: &str, ok: bool| {
        rows.iter().any(|r| r.0 == role && r.1 == raw && r.2 == ok)
    };
    assert!(has("model", "house.o3d", true), "{rows:?}");
    assert!(
        has("texture", "wall.bmp", true),
        "extension fallback: {rows:?}"
    );
    assert!(has("texture", "missing.bmp", false));
    assert!(
        has("texture", "asphalt.bmp", true),
        "global Texture/ fallback"
    );
    let _ = fs::remove_dir_all(&root);
}
