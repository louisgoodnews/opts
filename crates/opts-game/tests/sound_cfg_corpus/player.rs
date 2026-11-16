use super::*;

/// The player sound configurations (`[sound]` of every vehicle): how the definitions split by
/// what they do (M-0102).
#[test]
fn the_player_sound_configurations_split_by_playback() {
    use opts_core::formats::vehicles::BusManifest;
    use opts_game::sound_logic::{playback_of, Playback};
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let mut files = Vec::new();
    vehicle_files(&root.join("Vehicles"), &mut files);
    if files.is_empty() {
        eprintln!("no vehicles; skipping");
        return;
    }
    let install = opts_bevy::OmsiInstall::new(&root);
    let mut seen = std::collections::HashSet::new();
    let (mut cfgs, mut defs) = (0, 0);
    let mut by = std::collections::BTreeMap::<&str, usize>::new();
    let mut triggers = std::collections::HashSet::new();
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        let Ok(m) = BusManifest::parse_bytes(&bytes) else {
            continue;
        };
        let Some(rel) = m.sound_cfg.clone() else {
            continue;
        };
        let dir = f.parent().unwrap_or(Path::new("."));
        let Some(cfg_path) = install.find_in(dir, &rel) else {
            continue;
        };
        if !seen.insert(cfg_path.clone()) {
            continue;
        }
        cfgs += 1;
        let cfg = SoundConfig::parse(&latin1(&std::fs::read(&cfg_path).unwrap_or_default()));
        for d in &cfg.sounds {
            defs += 1;
            let key = match playback_of(d) {
                Playback::Loop => "loop",
                Playback::OnEdge => "edge",
                Playback::OnTrigger => "trigger",
                Playback::Unsupported => "unsupported",
            };
            *by.entry(key).or_default() += 1;
            for t in &d.triggers {
                triggers.insert(t.to_lowercase());
            }
        }
    }
    eprintln!(
        "{cfgs} player sound configurations, {defs} definitions: {by:?}; {} distinct trigger names",
        triggers.len()
    );
    assert!(
        cfgs >= 600 && defs >= 50_000,
        "{cfgs} configurations, {defs} definitions"
    );
    let unsupported = by.get("unsupported").copied().unwrap_or(0);
    assert!(
        unsupported * 10 < defs,
        "{unsupported} of {defs} definitions unsupported"
    );
}
