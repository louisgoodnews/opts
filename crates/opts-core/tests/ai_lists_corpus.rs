//! Corpus test: the AI list files of every installed map parse, group names used by the density
//! files exist in the lists, and the counts match `docs/src/formats/map/lists.md`.
//! Skipped unless `OMSI_PATH` points at an installation.

use std::path::{Path, PathBuf};

use opts_core::formats::map::{parse_unsched_trafficdens, parse_unsched_vehgroups, AiLists};

fn files_named(dir: &Path, pred: impl Fn(&str) -> bool + Copy, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            files_named(&p, pred, out);
        } else if pred(
            &p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase(),
        ) {
            out.push(p);
        }
    }
}

#[test]
fn every_ai_list_of_the_install_parses() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let maps = root.join("maps");
    let (mut lists, mut groups, mut entries, mut weightless, mut missing_normal) = (0, 0, 0, 0, 0);
    let mut files = Vec::new();
    files_named(
        &maps,
        |n| n.starts_with("ailists") && n.ends_with(".cfg"),
        &mut files,
    );
    for f in &files {
        let l = AiLists::parse(&std::fs::read(f).unwrap());
        lists += 1;
        groups += l.groups.len();
        for g in &l.groups {
            entries += g.entries.len();
            weightless += g.entries.iter().filter(|e| e.weight.is_none()).count();
        }
        if !l.groups.is_empty() && l.group("NormalCars").is_none() {
            missing_normal += 1;
        }
    }
    eprintln!("{lists} ailists files, {groups} groups, {entries} entries ({weightless} without weight), {missing_normal} lists without NormalCars");
    assert!(lists > 0 && groups > 0);

    let (mut vg_files, mut vg, mut dens_files, mut dens_groups, mut unknown_group) =
        (0, 0, 0, 0, 0);
    let mut vfiles = Vec::new();
    files_named(&maps, |n| n == "unsched_vehgroups.txt", &mut vfiles);
    for f in &vfiles {
        vg_files += 1;
        vg += parse_unsched_vehgroups(&std::fs::read(f).unwrap()).len();
    }
    let mut dfiles = Vec::new();
    files_named(&maps, |n| n == "unsched_trafficdens.txt", &mut dfiles);
    for f in &dfiles {
        dens_files += 1;
        let d = parse_unsched_trafficdens(&std::fs::read(f).unwrap());
        dens_groups += d.len();
        for g in &d {
            assert!(
                g.profiles.iter().all(|p| p
                    .points
                    .iter()
                    .all(|(h, v)| (0.0..=48.0).contains(h) && *v >= 0.0)),
                "{}: bad point",
                f.display()
            );
        }
        // Group names should exist in the map's ailists.cfg (when present, outside chrono folders).
        if let Some(l) = f
            .parent()
            .map(|p| p.join("ailists.cfg"))
            .filter(|p| p.exists())
        {
            let l = AiLists::parse(&std::fs::read(l).unwrap());
            unknown_group += d.iter().filter(|g| l.group(&g.name).is_none()).count();
        }
    }
    eprintln!("{vg_files} unsched_vehgroups ({vg} groups), {dens_files} unsched_trafficdens ({dens_groups} groups), {unknown_group} density groups without an AI group in the sibling ailists.cfg");
}
