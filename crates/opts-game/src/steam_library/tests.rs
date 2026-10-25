use super::*;

/// The layout Steam writes since 2021 (shortened from a real `libraryfolders.vdf`).
const CURRENT: &str = r#""libraryfolders"
{
	"0"
	{
		"path"		"/home/user/.local/share/Steam"
		"label"		""
		"apps"
		{
			"228980"		"748269574"
		}
	}
	"1"
	{
		"path"		"/mnt/games/SteamLibrary"
		"label"		""
		"apps"
		{
			"252530"		"212720608495"
			"951770"		"19946558325"
		}
	}
}
"#;

#[test]
fn the_current_format_lists_every_library_and_where_omsi_is() {
    assert_eq!(
        libraries(CURRENT),
        vec![
            Library {
                path: PathBuf::from("/home/user/.local/share/Steam"),
                has_omsi: false
            },
            Library {
                path: PathBuf::from("/mnt/games/SteamLibrary"),
                has_omsi: true
            },
        ]
    );
}

#[test]
fn the_format_before_2021_and_windows_escapes_are_read() {
    let old = r#"// written by Steam
"LibraryFolders"
{
	"TimeNextStatsReport"		"1612345678"
	"ContentStatsID"		"-123"
	"1"		"D:\\SteamLibrary"
	"2"		"E:\\Games\\Steam \"Fast\""
}
"#;
    let libs = libraries(old);
    let paths: Vec<_> = libs.iter().map(|l| l.path.clone()).collect();
    assert_eq!(
        paths,
        vec![
            PathBuf::from(r"D:\SteamLibrary"),
            PathBuf::from(r#"E:\Games\Steam "Fast""#)
        ]
    );
    assert!(libs.iter().all(|l| !l.has_omsi));
}

#[test]
fn malformed_text_gives_what_was_read() {
    assert!(libraries("").is_empty());
    assert!(libraries("\"libraryfolders\" \"oops\"").is_empty());
    let truncated = "\"libraryfolders\" { \"0\" { \"path\" \"/a\" } \"1\" { \"pa";
    assert_eq!(libraries(truncated).len(), 1);
}

#[test]
fn the_manifest_names_the_install_folder() {
    let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"252530\"\n\t\"name\"\t\t\"OMSI 2\"\n\t\"installdir\"\t\t\"OMSI 2\"\n}\n";
    assert_eq!(install_dir(acf), Some("OMSI 2".to_owned()));
    assert_eq!(install_dir("\"AppState\" { \"installdir\" \"\" }"), None);
    assert_eq!(install_dir("garbage"), None);
}

#[test]
fn reg_query_output_gives_the_path() {
    let out = "\r\nHKEY_CURRENT_USER\\Software\\Valve\\Steam\r\n    SteamPath    REG_SZ    c:/program files (x86)/steam\r\n\r\n";
    assert_eq!(
        registry_value(out, "SteamPath"),
        Some("c:/program files (x86)/steam".to_owned())
    );
    assert_eq!(registry_value(out, "InstallPath"), None);
    assert_eq!(registry_value("", "SteamPath"), None);
}

/// A scratch folder for one test.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("opts-steam-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

/// An OMSI 2 installation (`maps` and `Vehicles`) at `root`.
fn install(root: &Path) {
    std::fs::create_dir_all(root.join("maps/Grundorf")).expect("mkdir");
    std::fs::create_dir_all(root.join("Vehicles")).expect("mkdir");
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// `libraryfolders.vdf` text for libraries `(path, lists OMSI 2)`.
fn folders(libs: &[(&Path, bool)]) -> String {
    let mut s = String::from("\"libraryfolders\"\n{\n");
    for (i, (path, omsi)) in libs.iter().enumerate() {
        let apps = if *omsi {
            "\"252530\" \"1\""
        } else {
            "\"228980\" \"1\""
        };
        s.push_str(&format!(
            "\"{i}\" {{ \"path\" \"{}\" \"apps\" {{ {apps} }} }}\n",
            path.display().to_string().replace('\\', "\\\\")
        ));
    }
    s.push_str("}\n");
    s
}

#[test]
fn omsi_in_a_second_library_is_found_through_its_manifest() {
    let base = scratch("second");
    let steam = base.join("Steam");
    let lib = base.join("Library2");
    write(
        &steam.join("steamapps/libraryfolders.vdf"),
        &folders(&[(&steam, false), (&lib, true)]),
    );
    write(
        &lib.join("steamapps/appmanifest_252530.acf"),
        "\"AppState\" { \"appid\" \"252530\" \"installdir\" \"OMSI 2 Steam\" }",
    );
    install(&lib.join("steamapps/common/OMSI 2 Steam"));
    // Not an installation (no Vehicles): ignored.
    std::fs::create_dir_all(steam.join("steamapps/common/OMSI 2/maps")).expect("mkdir");
    assert_eq!(
        find_omsi(std::slice::from_ref(&steam)),
        Some(lib.join("steamapps/common/OMSI 2 Steam"))
    );
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn the_library_that_lists_omsi_wins_over_a_stray_copy() {
    let base = scratch("listed");
    let steam = base.join("Steam");
    let lib = base.join("Games");
    write(
        &steam.join("steamapps/libraryfolders.vdf"),
        &folders(&[(&steam, false), (&lib, true)]),
    );
    install(&steam.join("steamapps/common/OMSI 2"));
    install(&lib.join("steamapps/common/OMSI 2"));
    assert_eq!(
        find_omsi(std::slice::from_ref(&steam)),
        Some(lib.join("steamapps/common/OMSI 2"))
    );
    let _ = std::fs::remove_dir_all(base);
}

#[test]
fn without_library_files_the_default_folder_counts_and_nothing_is_none() {
    let base = scratch("plain");
    let steam = base.join("Steam");
    std::fs::create_dir_all(&steam).expect("mkdir");
    assert_eq!(find_omsi(std::slice::from_ref(&steam)), None);
    assert_eq!(find_omsi(&[base.join("missing")]), None);
    install(&steam.join("steamapps/common/OMSI 2"));
    assert_eq!(
        find_omsi(std::slice::from_ref(&steam)),
        Some(steam.join("steamapps/common/OMSI 2"))
    );
    let _ = std::fs::remove_dir_all(base);
}
