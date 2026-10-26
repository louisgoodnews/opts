//! Content test of the terrain shaped to aligned splines (`[spline_terrain_align]`,
//! `docs/src/architecture/terrain-carving.md` §10) on Berlin-Spandau and Winsenburg. Skipped
//! unless `OMSI_PATH` points at an installation with the map.
//!
//! The strips (`[heightprofile]`) set the terrain under a road however deep it lies, so they must
//! not put it over the drawn ground: at the centre of every drawn surface triangle of a tile that
//! an aligned strip covers, the strip lies at most a few centimetres above the surface.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use opts_bevy::install::OmsiInstall;
use opts_bevy::{spline_cut_triangles, spline_ground_triangles, TerrainCut};
use opts_core::formats::map::{MapManifest, MapTile};
use opts_core::formats::scenery::SplineProfile;

/// Drawn ground above which a strip counts as covering it (m).
const COVER_M: f32 = 0.05;

/// `(tiles with strips, sampled surface points under strips, points covered)` of a map.
fn measure(name: &str) -> Option<(usize, usize, usize)> {
    let root = std::env::var_os("OMSI_PATH").map(PathBuf::from)?;
    let install = OmsiInstall::new(&root);
    let dir = install.find_in(&root.join("maps"), name)?;
    let cfg = install.find_in(&dir, "global.cfg")?;
    let manifest = MapManifest::parse(&std::fs::read(cfg).ok()?).ok()?;
    let mut sli: HashMap<String, Option<Arc<SplineProfile>>> = HashMap::new();
    let (mut tiles, mut points, mut covered) = (0, 0, 0);
    for t in &manifest.tiles {
        let Some(tile) = install
            .find_in(&dir, &t.file_name)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| MapTile::parse(&b).ok())
        else {
            continue;
        };
        if tile.terrain_aligned.is_empty() {
            continue;
        }
        let (mut surfaces, mut strips) = (Vec::new(), Vec::new());
        for inst in &tile.splines {
            let profile = sli
                .entry(inst.sli_file.to_lowercase())
                .or_insert_with(|| {
                    let p = install.find_file(&inst.sli_file)?;
                    SplineProfile::parse(&std::fs::read(p).ok()?)
                        .ok()
                        .map(Arc::new)
                })
                .clone();
            let Some(profile) = profile else { continue };
            surfaces.extend(spline_ground_triangles(&profile, inst, (0.0, 0.0)));
            if tile.terrain_aligned.contains(&inst.id_code) {
                strips.extend(spline_cut_triangles(&profile, inst, (0.0, 0.0)));
            }
        }
        let cut = TerrainCut::new(strips, surfaces.clone(), 5.0);
        if cut.is_empty() {
            continue;
        }
        tiles += 1;
        for s in &surfaces {
            let c = (s[0] + s[1] + s[2]) / 3.0;
            if let Some(h) = cut.height(c.x, c.y) {
                points += 1;
                if h > c.z + COVER_M {
                    covered += 1;
                }
            }
        }
    }
    eprintln!("{name}: {tiles} tiles with strips, {points} surface points under strips, {covered} covered");
    Some((tiles, points, covered))
}

#[test]
fn aligned_strips_do_not_cover_the_drawn_ground() {
    for map in ["Winsenburg", "Berlin-Spandau"] {
        let Some((tiles, points, covered)) = measure(map) else {
            eprintln!("{map} not installed; skipped");
            continue;
        };
        assert!(tiles > 0, "{map} has aligned splines");
        assert!(points > 0, "{map}: strips lie under drawn ground");
        assert_eq!(
            covered, 0,
            "{map}: {covered} of {points} surface points under a strip are covered by it"
        );
    }
}
