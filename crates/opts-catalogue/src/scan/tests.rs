//! Tests of the filesystem walk.

use super::*;
use std::fs;

#[test]
fn scans_nested_tree_with_depths_and_extensions() {
    let dir = std::env::temp_dir().join(format!("opts_cat_scan_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("Vehicles/Bus1/model")).unwrap();
    fs::write(dir.join("Vehicles/Bus1/Bus1.BUS"), b"x").unwrap();
    fs::write(dir.join("Vehicles/Bus1/model/a.o3d.bak"), b"yy").unwrap();
    let out = scan_installation(&dir);
    assert!(out
        .dirs
        .iter()
        .any(|d| d.rel_path == "Vehicles/Bus1/model" && d.depth == 3));
    let bus = out.files.iter().find(|f| f.name == "Bus1.BUS").unwrap();
    assert_eq!(
        (bus.ext.as_str(), bus.size, bus.is_backup),
        ("bus", 1, false)
    );
    assert!(out.files.iter().any(|f| f.is_backup));
    assert_eq!(out.dirs[0].rel_path, "", "root first");
    let _ = fs::remove_dir_all(&dir);
}
