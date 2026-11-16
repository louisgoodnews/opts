//! Tests of the folder-change check: a changed top-level folder is seen cheaply.

use super::*;

/// A directory opened so its times can be set: Windows opens a directory only with
/// `FILE_FLAG_BACKUP_SEMANTICS` and needs `FILE_WRITE_ATTRIBUTES` access to change its times.
fn open_dir_for_times(dir: &std::path::Path) -> std::fs::File {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_WRITE_ATTRIBUTES: u32 = 0x0100;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        std::fs::OpenOptions::new()
            .access_mode(FILE_WRITE_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(dir)
            .unwrap()
    }
    #[cfg(not(windows))]
    {
        std::fs::File::open(dir).unwrap()
    }
}

#[test]
fn a_changed_top_level_folder_is_detected_cheaply() {
    let root = std::env::temp_dir().join(format!("opts-scanstatus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("Vehicles")).unwrap();
    let mtime = |p: &std::path::Path, secs: u64| {
        open_dir_for_times(p)
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap();
    };
    mtime(&root.join("Vehicles"), 1_000);
    mtime(&root, 1_000);
    assert!(!folder_changed_since(&root, 2_000));
    mtime(&root.join("Vehicles"), 3_000);
    assert!(
        folder_changed_since(&root, 2_000),
        "a new vehicle folder touches Vehicles"
    );
    mtime(&root.join("Vehicles"), 1_000);
    mtime(&root, 3_000);
    assert!(folder_changed_since(&root, 2_000));
    let _ = std::fs::remove_dir_all(&root);
}
