//! File classification and the events of the reload pipeline.

use std::path::{Path, PathBuf};

use bevy::prelude::{Message, Resource};

/// What kind of content a file holds; decides how it is validated and applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReloadKind {
    /// `.o3d`, `.x`
    Mesh,
    /// `.dds`, `.tga`, `.bmp`, `.png`, `.jpg`
    Texture,
    /// `tile_*.map`
    Tile,
    /// `*.terrain`
    Terrain,
    /// `.sco`
    Sco,
    /// `.sli`
    Sli,
    /// `.bus`
    Bus,
    /// `.cfg`
    Config,
    /// `.hum`
    Human,
    /// `.wav`
    Audio,
}

impl ReloadKind {
    /// Name used in logs and protocol replies.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Mesh => "mesh",
            Self::Texture => "texture",
            Self::Tile => "tile",
            Self::Terrain => "terrain",
            Self::Sco => "sco",
            Self::Sli => "sli",
            Self::Bus => "bus",
            Self::Config => "config",
            Self::Human => "human",
            Self::Audio => "audio",
        }
    }
}

/// The kind of a file by its extension; `None` for files the pipeline ignores.
#[must_use]
pub fn classify(path: &Path) -> Option<ReloadKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "o3d" | "x" => ReloadKind::Mesh,
        "dds" | "tga" | "bmp" | "png" | "jpg" | "jpeg" => ReloadKind::Texture,
        "map" => ReloadKind::Tile,
        "terrain" => ReloadKind::Terrain,
        "sco" => ReloadKind::Sco,
        "sli" => ReloadKind::Sli,
        "bus" => ReloadKind::Bus,
        "cfg" => ReloadKind::Config,
        "hum" => ReloadKind::Human,
        "wav" => ReloadKind::Audio,
        _ => return None,
    })
}

/// Whether a file name belongs to an editor's scratch files (swap, backup, lock, temp).
#[must_use]
pub fn is_editor_scratch(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    name.starts_with(".#")
        || name.starts_with('#')
        || name.ends_with('~')
        || [
            ".swp",
            ".swx",
            ".tmp",
            ".bak",
            ".orig",
            ".part",
            ".crdownload",
        ]
        .iter()
        .any(|s| name.ends_with(s))
        || (name.starts_with('.') && name.ends_with(".sw"))
}

/// A file changed (after debouncing) or a reload was requested explicitly.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ReloadRequested {
    /// The file.
    pub path: PathBuf,
    /// Its kind.
    pub kind: ReloadKind,
    /// Sequence number, unique per request (answers of `reload.*` use it).
    pub seq: u64,
}

/// The file parsed successfully; the application may now replace the affected content.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ReloadValidated {
    /// The file.
    pub path: PathBuf,
    /// Its kind.
    pub kind: ReloadKind,
    /// Sequence number of the request.
    pub seq: u64,
}

/// The file could not be used; nothing was changed.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ReloadFailed {
    /// The file.
    pub path: PathBuf,
    /// Its kind.
    pub kind: ReloadKind,
    /// Sequence number of the request.
    pub seq: u64,
    /// Why.
    pub error: String,
}

/// The application replaced the content that depended on the file.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ReloadApplied {
    /// The file.
    pub path: PathBuf,
    /// Its kind.
    pub kind: ReloadKind,
    /// Sequence number of the request.
    pub seq: u64,
    /// Human-readable list of what was rebuilt (`2 tiles`, `1 texture`, …).
    pub summary: String,
}

/// Asks the application to rebuild everything that is loaded (`reload.scene`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReloadSceneRequested {
    /// Sequence number of the request.
    pub seq: u64,
}

/// Final outcome of one request, kept for `reload.*` replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadOutcome {
    /// Request sequence number.
    pub seq: u64,
    /// The file (empty for scene reloads).
    pub path: PathBuf,
    /// `Ok(summary)` or `Err(error)`.
    pub result: Result<String, String>,
}

/// Counters and recent outcomes of the reload pipeline.
#[derive(Resource, Debug, Default)]
pub struct ReloadStats {
    /// Next sequence number.
    pub next_seq: u64,
    /// Requests seen.
    pub requested: u64,
    /// Requests applied.
    pub applied: u64,
    /// Requests that failed validation or application.
    pub failed: u64,
    /// The last [`ReloadStats::KEEP`] outcomes, oldest first.
    pub outcomes: std::collections::VecDeque<ReloadOutcome>,
}

impl ReloadStats {
    /// How many outcomes are kept.
    pub const KEEP: usize = 64;

    /// Allocates a sequence number.
    pub fn allocate(&mut self) -> u64 {
        self.next_seq += 1;
        self.next_seq
    }

    /// Records the end of a request.
    pub fn record(&mut self, outcome: ReloadOutcome) {
        if outcome.result.is_ok() {
            self.applied += 1;
        } else {
            self.failed += 1;
        }
        if self.outcomes.len() == Self::KEEP {
            self.outcomes.pop_front();
        }
        self.outcomes.push_back(outcome);
    }

    /// The outcome of request `seq`, if it has finished.
    #[must_use]
    pub fn outcome(&self, seq: u64) -> Option<&ReloadOutcome> {
        self.outcomes.iter().find(|o| o.seq == seq)
    }
}

/// Present when an application system consumes [`ReloadValidated`] and reports [`ReloadApplied`].
/// Without it the pipeline reports success right after validation.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct ReloadApplierRegistered;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_map_to_kinds() {
        let k = |p: &str| classify(Path::new(p));
        assert_eq!(k("a/b.O3D"), Some(ReloadKind::Mesh));
        assert_eq!(k("a.x"), Some(ReloadKind::Mesh));
        assert_eq!(k("t.dds"), Some(ReloadKind::Texture));
        assert_eq!(k("tile_1_2.map"), Some(ReloadKind::Tile));
        assert_eq!(k("tile_1_2.map.terrain"), Some(ReloadKind::Terrain));
        assert_eq!(k("m.cfg"), Some(ReloadKind::Config));
        assert_eq!(k("notes.txt"), None);
        assert_eq!(k("noext"), None);
    }

    #[test]
    fn editor_scratch_files_are_recognised() {
        for name in [
            "a.sco~",
            ".a.sco.swp",
            ".#a.sco",
            "#a.sco#",
            "a.sco.tmp",
            "a.o3d.bak",
        ] {
            assert!(is_editor_scratch(Path::new(name)), "{name}");
        }
        assert!(!is_editor_scratch(Path::new("model_main.cfg")));
    }

    #[test]
    fn stats_keep_a_bounded_history() {
        let mut s = ReloadStats::default();
        for i in 0..100 {
            let seq = s.allocate();
            s.record(ReloadOutcome {
                seq,
                path: PathBuf::from(format!("{i}")),
                result: Ok(String::new()),
            });
        }
        assert_eq!(s.outcomes.len(), ReloadStats::KEEP);
        assert!(s.outcome(100).is_some());
        assert!(s.outcome(1).is_none());
        assert_eq!(s.applied, 100);
    }
}
