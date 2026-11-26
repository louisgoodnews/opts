//! Reads the script files a vehicle or scenery script set names. `opts_sim` loads only bytes, so
//! the read happens here, in the caller.

use std::path::PathBuf;

use opts_sim::script::FileRead;

/// Reads the script file at `path` (`None` when the name did not resolve to a file).
pub(crate) fn read_script(path: Option<PathBuf>) -> FileRead {
    let Some(path) = path else {
        return FileRead::NotFound;
    };
    match std::fs::read(&path) {
        Ok(bytes) => FileRead::Loaded {
            name: path.to_string_lossy().into_owned(),
            bytes,
        },
        Err(e) => FileRead::Unreadable(e.to_string()),
    }
}
