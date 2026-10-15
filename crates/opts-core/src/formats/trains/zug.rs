//! `.zug` consist files: pairs of lines, a vehicle and `0` / `1` (see `docs/src/formats/text/zug.md`).

use thiserror::Error;

use super::{FormationSegment, FormationSource, Orientation, TrainFormation, VehicleRef};
use crate::formats::text::decode_bytes;

/// The longest formation accepted (the corpus maximum is 21 cars).
pub const MAX_CARS: usize = 256;

/// Why a file is not a consist.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ZugError {
    /// No lines at all.
    #[error("the file is empty")]
    Empty,
    /// The file has the shape of a script, not of a consist (it starts a block or a comment).
    #[error("line {line} looks like script text, not a consist entry")]
    NotAConsist {
        /// 1-based line.
        line: usize,
    },
    /// The second line of a pair is not `0` or `1`.
    #[error("line {line}: orientation is `{value}`, expected 0 or 1")]
    BadOrientation {
        /// 1-based line.
        line: usize,
        /// The text found.
        value: String,
    },
    /// A vehicle line is empty.
    #[error("line {line}: the vehicle name is empty")]
    EmptyVehicle {
        /// 1-based line.
        line: usize,
    },
    /// More than [`MAX_CARS`] cars.
    #[error("more than {MAX_CARS} cars")]
    TooLong,
}

/// Something worth telling the user that did not stop the parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZugNote {
    /// The last line had no orientation and was dropped.
    DroppedLastLine(String),
}

impl TrainFormation {
    /// Parses a `.zug` file. `name` is the display name, `path` the file's path relative to the
    /// OMSI root (kept in [`FormationSource::ZugFile`]).
    ///
    /// Accepts CRLF or LF, a missing final line break, trailing blank lines, UTF-8, UTF-16 and
    /// Windows-1252. Files that look like script text are refused.
    ///
    /// # Errors
    /// [`ZugError`] when the text is not a list of vehicle and orientation pairs.
    pub fn from_zug(
        bytes: &[u8],
        name: &str,
        path: &str,
    ) -> Result<(Self, Vec<ZugNote>), ZugError> {
        let text = decode_bytes(bytes);
        let mut lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        if lines.is_empty() {
            return Err(ZugError::Empty);
        }
        for (i, l) in lines.iter().enumerate() {
            let t = l.trim_start();
            if t.starts_with(['{', '\'']) || t.starts_with("(M.") || t.starts_with("(L.") {
                return Err(ZugError::NotAConsist { line: i + 1 });
            }
        }
        let mut notes = Vec::new();
        if lines.len() % 2 == 1 {
            let dropped = lines.pop().unwrap_or_default();
            notes.push(ZugNote::DroppedLastLine(dropped.trim().to_owned()));
            if lines.is_empty() {
                return Err(ZugError::Empty);
            }
        }
        if lines.len() / 2 > MAX_CARS {
            return Err(ZugError::TooLong);
        }
        let mut segments = Vec::with_capacity(lines.len() / 2);
        for (pair, chunk) in lines.chunks_exact(2).enumerate() {
            let vehicle = chunk[0].trim();
            if vehicle.is_empty() {
                return Err(ZugError::EmptyVehicle { line: pair * 2 + 1 });
            }
            let orientation = match chunk[1].trim() {
                "0" => Orientation::Forward,
                "1" => Orientation::Backward,
                other => {
                    return Err(ZugError::BadOrientation {
                        line: pair * 2 + 2,
                        value: other.to_owned(),
                    })
                }
            };
            segments.push(FormationSegment {
                vehicle: VehicleRef::from_entry(vehicle),
                orientation,
            });
        }
        Ok((
            Self {
                name: name.to_owned(),
                segments,
                source: FormationSource::ZugFile(path.to_owned()),
            },
            notes,
        ))
    }

    /// The `.zug` text of this formation (CRLF, no final line break, as OMSI writes it). Used by
    /// tests and by a future export; OPTS never writes into the installation.
    #[must_use]
    pub fn to_zug_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| format!("{}\r\n{}", s.vehicle.as_str(), s.orientation.zug_value()))
            .collect::<Vec<_>>()
            .join("\r\n")
    }
}
