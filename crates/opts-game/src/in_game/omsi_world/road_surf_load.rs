//! Finds and decodes the `.surf` height maps of the road profiles' textures (M-0146), once per
//! `.sli` and once per file.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use opts_core::formats::scenery::SplineProfile;

use super::road_surf::{strips_of, SurfMap, TexStrip};
use super::WorldLoadJob;

/// The names a file beside a texture may have (`suffix` `surf` or `cfg`): the reference plus the
/// suffix (`koppstein_1.dds.surf`), else the same stem with the other image extensions (the file
/// may be installed as `.dds` while the profile names a `.bmp`).
fn beside_names(texture: &str, suffix: &str) -> Vec<String> {
    let mut names = vec![format!("{texture}.{suffix}")];
    let stem = Path::new(texture).with_extension("");
    let stem = stem.to_string_lossy();
    for ext in ["dds", "bmp", "tga", "jpg", "png"] {
        let name = format!("{stem}.{ext}.{suffix}");
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// The `[surface]` type of a texture property file (asphalt, 0, when it names none).
fn surface_type(cfg: &str) -> u32 {
    let mut lines = cfg.lines().map(str::trim);
    while let Some(l) = lines.next() {
        if l.eq_ignore_ascii_case("[surface]") {
            return lines
                .find(|l| !l.is_empty())
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
        }
    }
    0
}

/// The strips with a height map of the profile `sli` (cached in the job).
pub(super) fn strips_for(
    job: &mut WorldLoadJob,
    sli: &str,
    profile: &SplineProfile,
) -> Arc<Vec<TexStrip>> {
    if let Some(s) = job.surf_strips.get(sli) {
        return s.clone();
    }
    // Spline textures live in the profile's own pack folder (`Splines/<Pack>/texture`), as the
    // ribbons' textures are found (`ContentLoader::texture`).
    let sli_dir = job
        .locate(sli)
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    let root = job.install.root().to_path_buf();
    let dirs = [
        sli_dir.join("texture"),
        sli_dir,
        job.config.map_dir.join("texture"),
        root.join("Texture"),
        root,
    ];
    let mut maps: Vec<Option<(Arc<SurfMap>, Option<f32>)>> = Vec::new();
    let mut authored = 0;
    for tex in &profile.textures {
        let map = surf_of_texture(job, &tex.file, &dirs);
        authored += usize::from(map.1.is_none());
        maps.push(Some(map));
    }
    let strips = Arc::new(strips_of(profile, |i| maps.get(i).cloned().flatten()));
    if authored > 0 {
        tracing::debug!(sli, authored, "road profile has surface height maps");
    }
    job.surf_strips.insert(sli.to_string(), strips.clone());
    strips
}

/// The height map of a texture found in `dirs`: its own `.surf`, else one made for its surface
/// type (`[surface]` of its `.cfg`, asphalt without one) with the length it repeats over.
pub(super) fn surf_of_texture(
    job: &mut WorldLoadJob,
    texture: &str,
    dirs: &[PathBuf],
) -> (Arc<SurfMap>, Option<f32>) {
    let find = |job: &WorldLoadJob, suffix: &str| {
        beside_names(texture, suffix)
            .iter()
            .find_map(|name| dirs.iter().find_map(|d| job.install.find_in(d, name)))
    };
    if let Some(m) = find(job, "surf").and_then(|p| load(job, p)) {
        return (m, None);
    }
    let surface = find(job, "cfg")
        .and_then(|p| std::fs::read(p).ok())
        .map_or(0, |b| surface_type(&String::from_utf8_lossy(&b)));
    let (map, period) = SurfMap::synthetic(surface);
    (Arc::new(map), Some(period))
}

/// Hands the strips of a tile's road profiles to the ground index (before the tile's roads).
pub(super) fn register(
    commands: &mut bevy::prelude::Commands,
    strips: Vec<(String, Arc<Vec<TexStrip>>)>,
) {
    commands.queue(move |world: &mut bevy::prelude::World| {
        if let Some(mut g) = world.get_resource_mut::<super::GroundIndex>() {
            for (sli, s) in strips {
                g.set_surf_strips(&sli, s);
            }
        }
    });
}

fn load(job: &mut WorldLoadJob, path: PathBuf) -> Option<Arc<SurfMap>> {
    job.surf_maps
        .entry(path)
        .or_insert_with_key(|p| {
            let map = std::fs::read(p).ok().and_then(|b| SurfMap::decode(&b));
            if map.is_none() {
                tracing::warn!(path = %p.display(), "unreadable .surf height map");
            }
            map.map(Arc::new)
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_surf_is_looked_for_under_the_reference_and_the_other_image_extensions() {
        let n = beside_names("koppstein_1.bmp", "surf");
        assert_eq!(n[0], "koppstein_1.bmp.surf");
        assert!(n.contains(&"koppstein_1.dds.surf".to_string()));
        assert_eq!(n.iter().filter(|s| *s == "koppstein_1.bmp.surf").count(), 1);
    }

    #[test]
    fn the_surface_type_is_read_from_the_texture_property_file() {
        assert_eq!(surface_type("[moisture]\n1\n\n[surface]\n2\n"), 2);
        assert_eq!(surface_type("\t[surface] tutorial\n[puddles]\n"), 0);
        assert_eq!(surface_type("[surface]\r\n\r\n15\r\n"), 15);
    }
}
