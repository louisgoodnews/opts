//! Corpus test of the `[time]` block of `laststn.osn` (`docs/src/formats/map/global-cfg.md`):
//! every installed map that has the file has a readable block inside the first
//! `OSN_TIME_PREFIX_BYTES`. Skipped unless `OMSI_PATH` points at an installation.

use std::io::Read;
use std::path::PathBuf;

use opts_core::formats::map::{OsnTime, OSN_TIME_PREFIX_BYTES};

#[test]
fn every_installed_laststn_osn_has_a_readable_time_block_in_its_prefix() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let (mut files, mut parsed, mut unreadable) = (0, 0, Vec::new());
    for entry in std::fs::read_dir(root.join("maps"))
        .into_iter()
        .flatten()
        .flatten()
    {
        let path = entry.path().join("laststn.osn");
        let Ok(file) = std::fs::File::open(&path) else {
            continue;
        };
        files += 1;
        let mut prefix = Vec::new();
        file.take(OSN_TIME_PREFIX_BYTES as u64)
            .read_to_end(&mut prefix)
            .expect("read prefix");
        match OsnTime::parse(&prefix) {
            Some(t) => {
                parsed += 1;
                assert!((1900..=2100).contains(&t.year), "{}: {t:?}", path.display());
            }
            None => unreadable.push(path.display().to_string()),
        }
    }
    eprintln!("laststn.osn: {files} files, {parsed} with a readable [time] block");
    assert!(
        unreadable.is_empty(),
        "no usable [time] block: {unreadable:?}"
    );
}
