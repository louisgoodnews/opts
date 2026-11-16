use super::*;

#[test]
fn every_scenery_sound_config_parses_cleanly() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let mut files = Vec::new();
    cfgs(&root.join("Sceneryobjects"), &mut files);
    if files.is_empty() {
        eprintln!("no scenery sound configs; skipping");
        return;
    }
    let (mut defs, mut warnings, mut no_pos) = (0, Vec::new(), 0);
    let (mut continuous, mut triggered, mut conditioned) = (0, 0, 0);
    for f in &files {
        let text = latin1(&std::fs::read(f).expect("read"));
        let c = SoundConfig::parse(&text);
        for w in &c.warnings {
            warnings.push(format!("{}: {w}", f.display()));
        }
        for d in &c.sounds {
            defs += 1;
            assert!(
                !d.file.is_empty(),
                "{}: a definition without a file",
                f.display()
            );
            if d.position.is_none() {
                eprintln!("  no [3d]: {} in {}", d.file, f.display());
            }
            no_pos += usize::from(d.position.is_none());
            continuous += usize::from(d.is_continuous());
            triggered += usize::from(!d.triggers.is_empty());
            conditioned += usize::from(!d.conditions.is_empty());
        }
    }
    eprintln!(
        "{} files, {defs} definitions ({continuous} continuous, {triggered} triggered, {conditioned} with a condition), {no_pos} without [3d], {} warnings",
        files.len(),
        warnings.len()
    );
    for w in warnings.iter().take(15) {
        eprintln!("  {w}");
    }
    assert!(
        files.len() >= 250 && defs >= 300,
        "{} files, {defs} definitions",
        files.len()
    );
    // One object (a taxi in `Kartoffelphantom`) names a file in the shared `Sounds` folder
    // without a [3d] block; every other definition has one.
    assert!(no_pos <= 1, "{no_pos} definitions without [3d]");
    assert!(warnings.len() <= 3, "{} warnings", warnings.len());
}
