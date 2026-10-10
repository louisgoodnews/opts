//! Writing the report files (never blocks the render loop: callers run it on a task pool).

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The two files of one saved report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedReport {
    /// The screenshot.
    pub png: PathBuf,
    /// The text report.
    pub report: PathBuf,
}

/// `<data dir>/opts/reports`, e.g. `~/.local/share/opts/reports` (Linux),
/// `%APPDATA%\opts\reports` (Windows), `~/Library/Application Support/opts/reports` (macOS).
#[must_use]
pub fn reports_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.data_dir().join("opts").join("reports"))
}

/// Creates the reports directory (and parents). Returns its path.
///
/// # Errors
/// Returns the I/O error if the directory cannot be created.
pub fn ensure_reports_dir() -> io::Result<PathBuf> {
    let dir = reports_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data directory"))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Writes `{stem}.png` and `{stem}.report` into `dir`, appending `-1`, `-2`, … to the stem when a
/// file of that name exists (two reports within one second). Existing files are never overwritten.
///
/// # Errors
/// Returns the first I/O error that is not a name collision.
pub fn save_report(dir: &Path, stem: &str, png: &[u8], report: &str) -> io::Result<SavedReport> {
    std::fs::create_dir_all(dir)?;
    for n in 0..1000u32 {
        let name = if n == 0 {
            stem.to_owned()
        } else {
            format!("{stem}-{n}")
        };
        let report_path = dir.join(format!("{name}.report"));
        let png_path = dir.join(format!("{name}.png"));
        let mut report_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&report_path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let mut png_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&png_path)
        {
            Ok(f) => f,
            Err(e) => {
                drop(report_file);
                let _ = std::fs::remove_file(&report_path);
                if e.kind() == io::ErrorKind::AlreadyExists {
                    continue;
                }
                return Err(e);
            }
        };
        let result = png_file
            .write_all(png)
            .and_then(|()| report_file.write_all(report.as_bytes()));
        if let Err(e) = result {
            let _ = std::fs::remove_file(&report_path);
            let _ = std::fs::remove_file(&png_path);
            return Err(e);
        }
        return Ok(SavedReport {
            png: png_path,
            report: report_path,
        });
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many reports with the same timestamp",
    ))
}
