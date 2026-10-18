//! Which vehicle files state which physical parameters (M-0073 data audit): mass, moments of
//! inertia, centre-of-gravity height, rolling resistance, axle spring/damper/force/wheel
//! diameter, and how plausible they are (rolling resistance against weight, stated yaw inertia
//! against a box estimate). Road vehicles only (`[type]` absent, 0 or 1).
//!
//! Usage: `dynamics_audit <omsi root>`
use std::path::{Path, PathBuf};

use opts_core::formats::vehicles::bus::BusManifest;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("bus") || x.eq_ignore_ascii_case("ovh"))
        {
            out.push(p);
        }
    }
}

fn quantiles(mut v: Vec<f32>) -> String {
    if v.is_empty() {
        return "none".into();
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |f: f32| v[((v.len() - 1) as f32 * f) as usize];
    format!(
        "n={} min {:.3} p10 {:.3} median {:.3} p90 {:.3} max {:.3}",
        v.len(),
        v[0],
        q(0.1),
        q(0.5),
        q(0.9),
        v[v.len() - 1]
    )
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("OMSI root"));
    let mut files = Vec::new();
    walk(&root.join("Vehicles"), &mut files);
    let (mut road, mut unreadable) = (0usize, 0usize);
    let mut count = std::collections::BTreeMap::<&str, usize>::new();
    let mut hit = |k: &'static str| *count.entry(k).or_default() += 1;
    let (mut mass, mut cg, mut roll, mut inertia_z, mut roll_per_weight, mut izz_ratio) = (
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let (mut spring, mut damper, mut maxf, mut dia, mut axles) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        let Ok(m) = BusManifest::parse_bytes(&bytes) else {
            unreadable += 1;
            continue;
        };
        if m.vehicle_type.unwrap_or(0) > 1 {
            continue;
        }
        road += 1;
        let ex = &m.extras;
        let mass_t = m.mass_tonnes.filter(|t| *t > 0.5);
        if let Some(t) = mass_t {
            hit("mass stated (> 0.5 t)");
            mass.push(t);
        }
        if m.axles.is_empty() {
            hit("no axles");
        } else {
            hit("axles stated");
            axles.push(m.axles.len() as f32);
        }
        if let Some(h) = ex.centre_of_gravity_height {
            hit("schwerpunkt");
            cg.push(h);
        }
        if let Some(r) = ex.roll_resistance_n {
            hit("rollwiderstand");
            roll.push(r);
            if let Some(t) = mass_t {
                roll_per_weight.push(r / (t * 1000.0 * 9.81));
            }
        }
        if ex.rotation_point_long.is_some() {
            hit("rot_pnt_long");
        }
        if let Some(i) = ex.inertia_t_m2 {
            hit("inertia (momentofintertia or [mass] lines 2-4)");
            inertia_z.push(i[2]);
            if let (Some(t), Some((size, _))) = (mass_t, m.bounding_box) {
                let est = t * (size.x * size.x + size.y * size.y) / 12.0;
                if est > 0.0 {
                    izz_ratio.push(i[2] / est);
                }
            }
        }
        for a in &m.axles {
            if let Some(v) = a.spring_rate.filter(|v| *v > 0.0) {
                spring.push(v);
            }
            if let Some(v) = a.damping.filter(|v| *v > 0.0) {
                damper.push(v);
            }
            if let Some(v) = a.max_force.filter(|v| *v > 0.0) {
                maxf.push(v);
            }
            if let Some(v) = a.wheel_diameter_m.filter(|v| *v > 0.1) {
                dia.push(v);
            }
        }
    }
    println!(
        "{} vehicle files, {road} road vehicles, {unreadable} unreadable",
        files.len()
    );
    for (k, v) in &count {
        println!(
            "  {k}: {v} ({:.0} %)",
            *v as f32 * 100.0 / road.max(1) as f32
        );
    }
    println!("mass t:                {}", quantiles(mass));
    println!("axles per vehicle:     {}", quantiles(axles));
    println!("cg height m:           {}", quantiles(cg));
    println!("rollwiderstand N:      {}", quantiles(roll));
    println!("rollwiderstand / (m g): {}", quantiles(roll_per_weight));
    println!("inertia Izz t m2:      {}", quantiles(inertia_z));
    println!("Izz / box estimate:    {}", quantiles(izz_ratio));
    println!("axle spring kN/m:      {}", quantiles(spring));
    println!("axle damper kNs/m:     {}", quantiles(damper));
    println!("axle max force kN:     {}", quantiles(maxf));
    println!("wheel diameter m:      {}", quantiles(dia));
}
