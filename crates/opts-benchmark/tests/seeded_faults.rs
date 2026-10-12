//! Seeded-fault acceptance test (M-0054): a synthetic, self-authored object is rendered through
//! the real worker pipeline. The clean object must pass; a mirrored heading, a 2x scale, a 10 m
//! offset, a removed texture and a removed mesh must each produce their category.
//!
//! The test needs a GPU adapter for Bevy's renderer; without one it reports "skipped" and passes.

use std::fs;
use std::path::Path;

use opts_benchmark::logging;
use opts_benchmark::scenery::worker::{Harness, WorkerConfig};
use opts_benchmark::scenery::{object, object_key, ObjectEntry, Record, Verdict};

const SCO: &str = "Sceneryobjects/Test/block.sco";

/// One box as double-sided quads (both windings, so culling never matters).
fn quads(min: [f32; 3], max: [f32; 3], out: &mut Vec<[[f32; 3]; 4]>) {
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
    out.extend(faces);
}

/// A hand-written text `.x`: an L-shaped solid (asymmetric, so mirrored headings differ).
fn block_x() -> String {
    let mut q = Vec::new();
    quads([-6.0, 0.0, -4.0], [6.0, 5.0, 4.0], &mut q);
    quads([2.0, 0.0, 4.0], [6.0, 8.0, 12.0], &mut q);
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut faces: Vec<String> = Vec::new();
    for quad in &q {
        let base = verts.len();
        verts.extend(quad.iter().copied());
        faces.push(format!("4;{},{},{},{}", base, base + 1, base + 2, base + 3));
        faces.push(format!("4;{},{},{},{}", base + 3, base + 2, base + 1, base));
    }
    let list = |items: Vec<String>| items.join(",\r\n").to_string() + ";";
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
    let ids = vec!["0"; faces.len()].join(",");
    format!(
        "xof 0303txt 0032\r\nMesh Block {{\r\n{};\r\n{}\r\n{};\r\n{};\r\n\
         MeshMaterialList {{\r\n1;\r\n{};\r\n{};;\r\nMaterial M {{\r\n1.0;1.0;1.0;1.0;;\r\n\
         3.2;\r\n0.0;0.0;0.0;;\r\n0.0;0.0;0.0;;\r\nTextureFilename {{ \"block.bmp\"; }}\r\n}}\r\n}}\r\n\
         MeshTextureCoords {{\r\n{};\r\n{}\r\n}}\r\n}}\r\n",
        verts.len(),
        v,
        faces.len(),
        list(faces.clone()),
        faces.len(),
        ids,
        verts.len(),
        uv,
    )
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// The synthetic installation: the entry anchor plus the test object.
fn build_root(root: &Path) {
    write(
        &root.join("Sceneryobjects/Generic/entrypoint_bus.sco"),
        b"[groups]\n1\nUtilities\n\n[friendlyname]\nanchor\n\n[entrypoint]\n\n[onlyeditor]\n",
    );
    write(
        &root.join(SCO),
        b"[groups]\n1\nTest\n\n[friendlyname]\nblock\n\n[mesh]\nblock.x\n",
    );
    write(
        &root.join("Sceneryobjects/Test/model/block.x"),
        block_x().as_bytes(),
    );
    let img = image::RgbImage::from_fn(8, 8, |x, y| {
        image::Rgb([60 + (x * 20) as u8, 120, 60 + (y * 20) as u8])
    });
    fs::create_dir_all(root.join("Sceneryobjects/Test/texture")).unwrap();
    img.save(root.join("Sceneryobjects/Test/texture/block.bmp"))
        .unwrap();
}

fn config(dir: &Path, root: &Path, fault: Option<&str>) -> WorkerConfig {
    WorkerConfig {
        run_dir: dir.to_path_buf(),
        id: 0,
        omsi_root: root.to_path_buf(),
        catalogue: None,
        anchor: opts_benchmark::scenery::mapgen::DEFAULT_ANCHOR.into(),
        max_objects: 100,
        object_timeout: std::time::Duration::from_secs(60),
        keep_images: "none".into(),
        seed_fault: fault.map(String::from),
        thresholds: opts_benchmark::scenery::verdict::Thresholds::default(),
    }
}

fn run(h: &mut Harness, fault: Option<&str>) -> Record {
    h.config.seed_fault = fault.map(String::from);
    let entry = ObjectEntry {
        sco: SCO.into(),
        key: object_key(SCO),
        tier: "unreferenced".into(),
    };
    object::process(h, &entry).0
}

#[test]
fn seeded_faults_are_caught() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("omsi");
    build_root(&root);
    let sink = logging::init(true);
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Harness::new(config(&tmp.path().join("run"), &root, None), sink)
    }));
    let mut h = match built {
        Ok(Ok(h)) => h,
        other => {
            eprintln!(
                "skipped: no usable GPU adapter ({:?})",
                other.err().is_some()
            );
            return;
        }
    };
    opts_bevy::texture_sentinel::set_missing_texture_sentinel(true);

    let clean = run(&mut h, None);
    assert_eq!(
        clean.verdict,
        Verdict::Pass,
        "clean object: {:?}",
        clean.categories
    );

    for (fault, category) in [
        ("heading-mirror", "F-XFORM-ROTATION"),
        ("scale-2x", "F-XFORM-SCALE"),
        ("offset-10m", "F-XFORM-OFFSET"),
    ] {
        let r = run(&mut h, Some(fault));
        assert_eq!(r.verdict, Verdict::Fail, "{fault}");
        assert!(
            r.categories.iter().any(|c| c == category),
            "{fault}: {:?}",
            r.categories
        );
    }

    let tex = root.join("Sceneryobjects/Test/texture/block.bmp");
    fs::remove_file(&tex).unwrap();
    let r = run(&mut h, None);
    assert!(
        r.categories.iter().any(|c| c == "F-TEX-MISSING"),
        "{:?}",
        r.categories
    );

    fs::remove_file(root.join("Sceneryobjects/Test/model/block.x")).unwrap();
    let r = run(&mut h, None);
    assert_eq!(r.verdict, Verdict::Fail);
    assert!(
        r.categories.iter().any(|c| c == "F-MESH-MISSING"),
        "{:?}",
        r.categories
    );
}
