//! Corpus test: the AI sound configurations of the installation yield playable voices.
use super::*;

/// The AI sound configurations of the installation (`[sound_ai]`, or a `[sound]` that names a
/// shared `Sounds/AI_Cars` file): how much of them the traffic model can play (M-0140).
#[test]
fn the_ai_sound_configurations_yield_playable_voices() {
    use opts_core::formats::vehicles::BusManifest;
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
    let (mut cfgs, mut defs, mut playable, mut empty) = (0, 0, 0, 0);
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        let Ok(m) = BusManifest::parse_bytes(&bytes) else {
            continue;
        };
        let rel = m.sound_ai_cfg.clone().or_else(|| {
            m.sound_cfg.clone().filter(|s| {
                s.replace('\\', "/")
                    .to_lowercase()
                    .contains("sounds/ai_cars")
            })
        });
        let Some(rel) = rel else { continue };
        let dir = f.parent().unwrap_or(Path::new("."));
        let Some(cfg_path) = install.find_in(dir, &rel) else {
            continue;
        };
        if !seen.insert(cfg_path.clone()) {
            continue;
        }
        cfgs += 1;
        let cfg = SoundConfig::parse(&latin1(&std::fs::read(&cfg_path).unwrap_or_default()));
        defs += cfg.sounds.len();
        let set = opts_game::vehicle_sound::choose(&cfg, |_| Some(PathBuf::from("x.wav")));
        playable += set.voices.len();
        empty += usize::from(set.voices.is_empty());
    }
    eprintln!("{cfgs} AI sound configurations, {defs} definitions, {playable} playable (cap {} each), {empty} configurations without any", opts_game::vehicle_sound::MAX_AI_VOICES);
    assert!(cfgs >= 300, "{cfgs} configurations");
    assert!(playable >= 3000, "{playable} playable voices");
    assert!(
        empty * 8 < cfgs,
        "{empty} of {cfgs} configurations stay silent"
    );
}
