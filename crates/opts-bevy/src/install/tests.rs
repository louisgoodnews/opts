//! Tests of the installation paths: case-insensitive lookup, folder case, and symlinked roots.

use super::*;

#[test]
fn install_lookup_ignores_case_and_separators() {
    let dir = std::env::temp_dir().join(format!("opts_install_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Sceneryobjects/Foo")).unwrap();
    std::fs::write(dir.join("Sceneryobjects/Foo/Bar.sco"), b"x").unwrap();
    let install = OmsiInstall::new(&dir);
    assert!(install.find_file("sceneryobjects\\foo\\bar.SCO").is_some());
    assert!(install
        .find_file("sceneryobjects\\foo\\missing.sco")
        .is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn find_in_resolves_folder_case_of_the_base() {
    let dir = std::env::temp_dir().join(format!("opts_install_case_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Sceneryobjects/Foo/Texture")).unwrap();
    std::fs::write(dir.join("Sceneryobjects/Foo/Texture/Wall.jpg"), b"x").unwrap();
    let install = OmsiInstall::new(&dir);
    // The loader builds `<sco dir>/texture`; the folder on disk is `Texture`.
    let base = dir.join("Sceneryobjects/Foo/texture");
    assert!(install.find_in(&base, "wall.JPG").is_some());
    assert!(install.find_in(&base, "other.jpg").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn find_in_uses_the_index_through_a_symlinked_root() {
    let dir = std::env::temp_dir().join(format!("opts_install_link_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let real = dir.join("real");
    std::fs::create_dir_all(real.join("Sceneryobjects/Foo/Texture")).unwrap();
    std::fs::write(real.join("Sceneryobjects/Foo/Texture/Wall.jpg"), b"x").unwrap();
    std::os::unix::fs::symlink(&real, dir.join("link")).unwrap();
    let canonical = real.canonicalize().unwrap();
    // An index that, like the catalogue, answers with paths below the resolved root.
    let indexed = canonical.clone();
    let lookup: PathLookup = Arc::new(move |rel: &str| {
        (rel.to_lowercase() == "sceneryobjects/foo/texture/wall.jpg")
            .then(|| indexed.join("Sceneryobjects/Foo/Texture/Wall.jpg"))
    });
    let install = OmsiInstall::new(dir.join("link")).with_lookup(lookup);
    // The base is spelled below the resolved root and has the wrong folder case.
    let base = canonical.join("Sceneryobjects/Foo/texture");
    let found = install.find_in(&base, "Wall.jpg");
    assert_eq!(
        found,
        Some(canonical.join("Sceneryobjects/Foo/Texture/Wall.jpg"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}
