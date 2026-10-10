//! Prints, for the splines of one tile, the profile geometry and the terrain height under the centreline.
use opts_core::formats::map::{MapTerrain, MapTile};
use opts_core::formats::scenery::SplineProfile;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (map, tile) = (&args[1], &args[2]);
    let root = std::path::Path::new(map)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let base = format!("{map}/tile_{tile}.map");
    let t = MapTile::parse(&std::fs::read(&base).unwrap()).unwrap();
    let ter = MapTerrain::parse(&std::fs::read(format!("{base}.terrain")).unwrap()).unwrap();
    let dim = ter.grid_cells as usize + 1;
    let mut seen = std::collections::HashSet::new();
    for s in &t.splines {
        let h = |x: f32, y: f32| {
            let (ix, iy) = (
                ((x / 5.0) as usize).min(dim - 2),
                ((y / 5.0) as usize).min(dim - 2),
            );
            let (fx, fy) = (x / 5.0 - ix as f32, y / 5.0 - iy as f32);
            let g = |i: usize, j: usize| ter.heights[j * dim + i];
            g(ix, iy) * (1.0 - fx) * (1.0 - fy)
                + g(ix + 1, iy) * fx * (1.0 - fy)
                + g(ix, iy + 1) * (1.0 - fx) * fy
                + g(ix + 1, iy + 1) * fx * fy
        };
        let th = h(
            s.position.x.clamp(0.0, 299.0),
            s.position.y.clamp(0.0, 299.0),
        );
        println!(
            "{:60} z={:8.3} terrain={:8.3} dz={:7.3} mirror={} len={:.0}",
            s.sli_file,
            s.position.z,
            th,
            s.position.z - th,
            s.mirrored,
            s.length_m
        );
        if seen.insert(s.sli_file.clone()) {
            let p = root.join(s.sli_file.replace('\\', "/"));
            if let Ok(b) = std::fs::read(&p) {
                if let Ok(prof) = SplineProfile::parse(&b) {
                    for gp in &prof.graphical_profiles {
                        let pts: Vec<String> = gp
                            .points
                            .iter()
                            .map(|p| format!("({:.2},{:.2})", p.x, p.z))
                            .collect();
                        println!("    profile tex{}: {}", gp.texture_index, pts.join(" "));
                    }
                    for hp in &prof.height_profiles {
                        println!(
                            "    heightprofile x {:.1}..{:.1} z {:.2}..{:.2}",
                            hp.x_start, hp.x_end, hp.z_start, hp.z_end
                        );
                    }
                }
            }
        }
    }
}
