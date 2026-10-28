//! Markdown table and top-object report for the instancing inventory example.

use super::Totals;

pub(super) fn report(map: &str, t: &Totals, top: usize) {
    let handle_meshes: usize = t.handle_keys.values().sum();
    let mut per_tile: Vec<f64> = t
        .per_tile
        .iter()
        .filter(|(p, _)| *p > 0)
        .map(|(p, u)| *p as f64 / (*u).max(1) as f64)
        .collect();
    per_tile.sort_by(f64::total_cmp);
    let median = per_tile.get(per_tile.len() / 2).copied().unwrap_or(0.0);
    let repeat_map = t.parts as f64 / t.pair_keys.len().max(1) as f64;
    println!("### {map}\n");
    println!("| Item | Value |\n| :--- | ---: |");
    println!("| tiles read | {} |", t.tiles);
    println!(
        "| placed objects (all / trees / vehicle-model / no mesh / sco missing) | {} / {} / {} / {} / {} |",
        t.objects, t.tree_objects, t.vehicle_objects, t.no_mesh, t.missing_sco
    );
    println!(
        "| parts (mesh entities: slot with triangles, per placed object) | {} |",
        t.parts
    );
    println!(
        "| parts in flat static objects / LOD-only objects / dynamic objects | {} / {} / {} |",
        t.parts_flat, t.parts_lod_only, t.parts_dynamic
    );
    println!(
        "| of which tree billboards / parts of objects with a rendered text texture | {} / {} |",
        t.parts_trees, t.parts_text
    );
    println!(
        "| unique mesh files / unique mesh contents | {} / {} |",
        t.mesh_paths.len(),
        t.mesh_content.len()
    );
    println!(
        "| current handle keys (.o3d + `[matl]`) → unique mesh handles (parts) | {} → {} |",
        t.handle_keys.len(),
        handle_meshes
    );
    println!("| unique material contents | {} |", t.material_keys.len());
    println!(
        "| unique (mesh part, material) pairs: all / flat static only | {} / {} |",
        t.pair_keys.len(),
        t.pair_keys_flat.len()
    );
    println!(
        "| repeat factor parts / pairs: whole map; median per tile | {:.1} ; {:.1} |",
        repeat_map, median
    );
    println!("| triangles drawn (all parts) | {} |", t.triangles);
    println!(
        "| parts of scrambled (unreadable) meshes | {} |",
        t.scrambled_parts
    );
    let mut best: Vec<_> = t.per_sco.iter().collect();
    best.sort_by_key(|(_, (_, p))| std::cmp::Reverse(*p));
    println!("\nMost repeated objects (placed / parts):\n");
    for (sco, (n, p)) in best.into_iter().take(top) {
        println!("* `{}` — {n} / {p}", sco.replace('\\', "/"));
    }
    println!();
}
