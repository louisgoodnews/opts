//! the vehicle loader and the register and random scaling cases.

use super::*;

#[test]
fn vehicle_loader_reads_scripts_varlists_and_constfiles() {
    use opts_core::formats::vehicles::BusManifest;
    use opts_sim::script::{load_vehicle_scripts, FileRead};

    let dir = std::env::temp_dir().join(format!("opts-script-load-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("main.osc"),
        "{frame}\n(C.L.k) (L.L.in) * (S.L.out)\n{end}",
    )
    .expect("write");
    std::fs::write(dir.join("v.txt"), "in\nout\n").expect("write");
    std::fs::write(dir.join("c.txt"), "prose\n[const]\nk\n3\n").expect("write");
    let bus = BusManifest::parse(
        "[varnamelist]\n1\nv.txt\n[script]\n2\nmain.osc\nmissing.osc\n[constfile]\n1\nc.txt\n",
    )
    .expect("bus");
    let (mut e, diags) = load_vehicle_scripts(&bus, |rel| {
        let p = dir.join(rel);
        match p.exists().then_some(p) {
            None => FileRead::NotFound,
            Some(p) => FileRead::Loaded {
                name: p.to_string_lossy().into_owned(),
                bytes: std::fs::read(&p).expect("fixture"),
            },
        }
    });
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.message == "file not found")
            .count(),
        1
    );
    assert!(e.undeclared_variables().is_empty());
    e.set("in", 4.0);
    e.run_frame(0.016, false);
    assert_eq!(e.get("out"), 12.0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn t_osc_001c_register_store_keeps_the_value() {
    // `5 s0 3 +` leaves 8 (5 stays below the 3 it is added to... the store did not pop).
    let e = run("2 s0 3 + (S.L.x) l0 (S.L.y)");
    assert_eq!(e.get("x"), 5.0);
    assert_eq!(e.get("y"), 2.0);
    let e = run("4 s1 (L.L.z) + (S.L.x)");
    assert_eq!(
        e.get("x"),
        4.0,
        "the stored 4 is still there to add to z (= 0)"
    );
}

#[test]
fn random_scales_by_the_value_below_it() {
    let e = run("1000 random (S.L.x)");
    assert!((0.0..1000.0).contains(&e.get("x")));
    let e = run("0 random (S.L.x)");
    assert_eq!(e.get("x"), 0.0);
}
