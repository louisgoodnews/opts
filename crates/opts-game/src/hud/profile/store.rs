//! Loading HUD profiles from the shipped and user folders.

use super::*;

/// All profiles, by kind.
#[derive(Resource, Debug, Clone, Default)]
pub struct HudProfiles(pub BTreeMap<VehicleKind, HudProfile>);

impl HudProfiles {
    /// The shipped profiles with the overrides of `dir` (the `hud/` folder of the configuration).
    #[must_use]
    pub fn load(dir: Option<&Path>) -> Self {
        let mut map = BTreeMap::new();
        for kind in VehicleKind::ALL {
            let user = dir.and_then(|d| read_user(&d.join(format!("{}.ron", kind.id())), kind));
            let profile = user.unwrap_or_else(|| {
                let (p, problems) = HudProfile::parse(shipped(kind))
                    .unwrap_or_else(|e| panic!("shipped profile {}: {e}", kind.id()));
                debug_assert!(problems.is_empty(), "shipped profile {}", kind.id());
                p
            });
            map.insert(kind, profile);
        }
        Self(map)
    }

    /// The profile of `kind`.
    #[must_use]
    pub fn get(&self, kind: VehicleKind) -> Option<&HudProfile> {
        self.0.get(&kind)
    }
}

fn read_user(path: &Path, kind: VehicleKind) -> Option<HudProfile> {
    let text = std::fs::read_to_string(path).ok()?;
    match HudProfile::parse(&text) {
        Ok((p, problems)) if p.kind == kind => {
            for pr in problems {
                tracing::warn!(file = %path.display(), "HUD profile: {pr}");
            }
            Some(p)
        }
        Ok(_) => {
            tracing::warn!(file = %path.display(), "HUD profile is for another vehicle kind, ignored");
            None
        }
        Err(e) => {
            tracing::warn!(file = %path.display(), "HUD profile ignored: {e}");
            None
        }
    }
}

/// The folder user profiles are read from (`<config dir>/hud`).
#[must_use]
pub fn user_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "opts", "opts").map(|d| d.config_dir().join("hud"))
}

/// Widgets that no shipped profile lists (a registry entry without a place is a mistake).
#[must_use]
pub fn unplaced(profiles: &HudProfiles) -> Vec<&'static str> {
    WIDGETS
        .iter()
        .filter(|w| {
            !profiles
                .0
                .values()
                .any(|p| p.widgets.iter().any(|e| e.id == w.id))
        })
        .map(|w| w.id)
        .collect()
}
