//! Parses a changed file completely before anything is replaced ("validate, then apply").

use std::path::Path;

use opts_core::formats::humans::HumanProfile;
use opts_core::formats::map::{MapManifest, MapTerrain, MapTile};
use opts_core::formats::scenery::{SceneryObject, SplineProfile};
use opts_core::formats::text::decode_bytes;
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

use super::events::ReloadKind;

/// Whether `path` holds a usable file of `kind`.
///
/// # Errors
/// A readable description: unreadable, empty (typical for a half-written save) or the parser's message.
pub fn validate(path: &Path, kind: ReloadKind) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if bytes.is_empty() {
        return Err("the file is empty".into());
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let described = |e: &dyn std::fmt::Display| e.to_string();
    match kind {
        ReloadKind::Mesh => crate::mesh::parse_mesh_file(path, &bytes).map(|_| ()),
        ReloadKind::Texture => crate::texture::load_bevy_image(&bytes)
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Tile => MapTile::parse(&bytes)
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Terrain => MapTerrain::parse(&bytes)
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Sco => SceneryObject::parse(&bytes)
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Sli => SplineProfile::parse(&bytes)
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Bus => BusManifest::parse_bytes(&bytes)
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Human => HumanProfile::from_text(&decode_bytes(&bytes))
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Config if name.starts_with("model") => ModelConfig::parse_bytes(&bytes)
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Config if name == "global.cfg" => MapManifest::parse(&bytes)
            .map(|_| ())
            .map_err(|e| described(&e)),
        ReloadKind::Config | ReloadKind::Audio => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("opts_validate_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let p = dir.join(name);
        std::fs::write(&p, bytes).expect("write");
        p
    }

    #[test]
    fn empty_and_missing_files_fail() {
        let empty = temp("e.sco", b"");
        assert!(validate(&empty, ReloadKind::Sco)
            .unwrap_err()
            .contains("empty"));
        assert!(
            validate(Path::new("/definitely/missing.sco"), ReloadKind::Sco)
                .unwrap_err()
                .contains("cannot read")
        );
    }

    #[test]
    fn garbage_meshes_and_textures_fail() {
        let mesh = temp("g.o3d", b"not a mesh at all");
        assert!(validate(&mesh, ReloadKind::Mesh).is_err());
        let tex = temp("g.png", b"not an image");
        assert!(validate(&tex, ReloadKind::Texture).is_err());
    }

    #[test]
    fn plain_text_configs_pass() {
        let sco = temp("ok.sco", b"[friendlyname]\nA test object\n");
        assert!(validate(&sco, ReloadKind::Sco).is_ok());
        let cfg = temp("notes.cfg", b"anything");
        assert!(validate(&cfg, ReloadKind::Config).is_ok());
    }
}
