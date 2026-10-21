//! OMSI's Windows paths on this file system (`STLoadTex`): backslashes, `..` and letter case.

use std::fs;
use std::path::Path;

use opts_bevy::script_text::resolve_path;

fn tree(name: &str) -> std::path::PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Vehicles/Bus/texture")).expect("dirs");
    fs::create_dir_all(root.join("Vehicles/Anzeigen/Krueger/230x32")).expect("dirs");
    fs::write(root.join("Vehicles/Anzeigen/Krueger/230x32/Ziel.bmp"), b"x").expect("file");
    root
}

#[test]
fn a_script_path_climbs_out_of_the_texture_folder_and_ignores_letter_case() {
    let root = tree("script_text_paths_a");
    // The base names `Texture`, the folder on disk is `texture`; the file is `Ziel.bmp`.
    let base = root.join("Vehicles/Bus/Texture");
    let found = resolve_path(&base, "..\\..\\anzeigen\\KRUEGER\\230x32\\ziel.BMP").expect("found");
    assert_eq!(
        found,
        root.join("Vehicles/Anzeigen/Krueger/230x32/Ziel.bmp")
    );
    assert!(found.exists());
}

#[test]
fn a_missing_file_is_none() {
    let root = tree("script_text_paths_b");
    assert!(resolve_path(&root.join("Vehicles/Bus/texture"), "nothing\\here.bmp").is_none());
}
