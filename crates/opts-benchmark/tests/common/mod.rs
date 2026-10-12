//! Shared helpers of the integration tests: a self-authored installation made of hand-written
//! `.x` boxes, solid-colour textures, `.sco` files and `.osc` scripts. Nothing of OMSI is used.

#![allow(dead_code)]

use std::fs;
use std::path::Path;

use image::{Rgb, RgbImage};
use opts_benchmark::scenery::worker::WorkerConfig;

/// A box as double-sided quads (both windings, so culling never matters), as `.x` text.
pub fn box_x(min: [f32; 3], max: [f32; 3], texture: &str) -> String {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let faces = [
        [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
        [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]],
        [[x0, y0, z0], [x0, y1, z0], [x0, y1, z1], [x0, y0, z1]],
        [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
    ];
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut tris: Vec<String> = Vec::new();
    for quad in &faces {
        let base = verts.len();
        verts.extend(quad.iter().copied());
        tris.push(format!("4;{},{},{},{}", base, base + 1, base + 2, base + 3));
        tris.push(format!("4;{},{},{},{}", base + 3, base + 2, base + 1, base));
    }
    let list = |items: Vec<String>| items.join(",\r\n") + ";";
    let v = list(
        verts
            .iter()
            .map(|p| format!("{:.3};{:.3};{:.3};", p[0], p[1], p[2]))
            .collect(),
    );
    let uv = list(
        (0..verts.len())
            .map(|i| format!("{}.0;0.0;", i % 2))
            .collect(),
    );
    let ids = vec!["0"; tris.len()].join(",");
    format!(
        "xof 0303txt 0032\r\nMesh Box {{\r\n{};\r\n{}\r\n{};\r\n{};\r\n\
         MeshMaterialList {{\r\n1;\r\n{};\r\n{};;\r\nMaterial M {{\r\n1.0;1.0;1.0;1.0;;\r\n\
         3.2;\r\n0.0;0.0;0.0;;\r\n0.0;0.0;0.0;;\r\nTextureFilename {{ \"{texture}\"; }}\r\n}}\r\n}}\r\n\
         MeshTextureCoords {{\r\n{};\r\n{}\r\n}}\r\n}}\r\n",
        verts.len(),
        v,
        tris.len(),
        list(tris.clone()),
        tris.len(),
        ids,
        verts.len(),
        uv,
    )
}

/// Writes `bytes` to `path`, creating folders.
pub fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// Writes a mesh `name.x` of one box with a solid-colour texture `name.bmp` into the folder
/// `Sceneryobjects/Test/`.
pub fn add_box(root: &Path, name: &str, min: [f32; 3], max: [f32; 3], rgb: [u8; 3]) {
    let tex = format!("{name}.bmp");
    write(
        &root.join(format!("Sceneryobjects/Test/model/{name}.x")),
        box_x(min, max, &tex).as_bytes(),
    );
    let img = RgbImage::from_pixel(8, 8, Rgb(rgb));
    fs::create_dir_all(root.join("Sceneryobjects/Test/texture")).unwrap();
    img.save(root.join(format!("Sceneryobjects/Test/texture/{tex}")))
        .unwrap();
}

/// Writes `Sceneryobjects/Test/<name>.sco`.
pub fn add_sco(root: &Path, name: &str, text: &str) {
    write(
        &root.join(format!("Sceneryobjects/Test/{name}.sco")),
        text.as_bytes(),
    );
}

/// Writes `Sceneryobjects/Test/script/<name>`.
pub fn add_script(root: &Path, name: &str, text: &str) {
    write(
        &root.join(format!("Sceneryobjects/Test/script/{name}")),
        text.as_bytes(),
    );
}

/// The entry anchor every generated map needs.
pub fn add_anchor(root: &Path) {
    write(
        &root.join("Sceneryobjects/Generic/entrypoint_bus.sco"),
        b"[groups]\n1\nUtilities\n\n[friendlyname]\nanchor\n\n[entrypoint]\n\n[onlyeditor]\n",
    );
}

/// A worker configuration for the tests.
pub fn config(dir: &Path, root: &Path, fault: Option<&str>, scripts: bool) -> WorkerConfig {
    WorkerConfig {
        run_dir: dir.to_path_buf(),
        id: 0,
        omsi_root: root.to_path_buf(),
        catalogue: None,
        anchor: opts_benchmark::scenery::mapgen::DEFAULT_ANCHOR.into(),
        max_objects: 100,
        object_timeout: std::time::Duration::from_secs(120),
        keep_images: "none".into(),
        scripts,
        seed_fault: fault.map(String::from),
        thresholds: opts_benchmark::scenery::verdict::Thresholds::default(),
    }
}
