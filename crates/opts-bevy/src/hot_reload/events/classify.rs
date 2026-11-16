//! Classifying changed files: the reload kind of an extension and the editor scratch files.

use super::*;

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
