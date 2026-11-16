//! The view of the chosen map: its resolved profile, the saved profiles, the presets and the advice.

use super::*;

fn map_context(world: &World) -> Option<(MapKey, String, std::path::PathBuf, Vec<String>)> {
    let setup = world.get_resource::<NewGameSetup>()?;
    let catalogue = world.get_resource::<GameCatalogue>();
    let args = world.get_resource::<AppArgs>();
    let key = key_of(setup, catalogue, args)?;
    let dir = args.and_then(|a| a.0.map_path.clone()).or_else(|| {
        let id = catalogue
            .and_then(|c| c.map_by_name(&setup.selected_map))
            .map_or_else(|| setup.selected_map.clone(), |m| m.id.clone());
        Some(crate::omsi_root::known_omsi_root()?.join("maps").join(id))
    })?;
    let name = dir.file_name()?.to_string_lossy().into_owned();
    let chrono = active_chrono(&dir, setup);
    Some((key, name, dir, chrono))
}

/// The view of the plan's map; starts the static count the first time.
pub fn view(world: &mut World) -> Option<ProfileView> {
    let (key, name, dir, chrono_entries) = map_context(world)?;
    let store = world.resource::<ProfileStoreRes>().clone();
    world.resource_scope(|world, mut statics: Mut<StaticProfiles>| {
        request_for(&mut statics, &store, &key.map_id, dir);
        let registry = world.resource::<SettingsRegistry>();
        let (resolved, saved, last) = match &store.0 {
            Some(s) => (
                s.resolve(registry, &key),
                s.list_map(registry, &key.map_id),
                s.last_session(&key),
            ),
            None => (
                Resolved {
                    profile: None,
                    source: ProfileSource::Global,
                    dropped: Vec::new(),
                },
                Vec::new(),
                None,
            ),
        };
        let stat = statics.get(&key.map_id);
        let load = stat.map_or_else(MapLoadProfile::default, |s| {
            s.to_load_profile(
                last.as_ref().map(|m| m.avg_ms),
                last.as_ref().map(|m| m.p95_ms),
            )
        });
        let traffic = world
            .resource::<GameSettings>()
            .raw("gameplay.traffic")
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);
        let advice = if stat.is_some() {
            advise(&load, traffic)
        } else {
            Vec::new()
        };
        let picked = world
            .resource::<MapProfilePick>()
            .0
            .as_ref()
            .filter(|p| super::super::profile::slug(&p.map) == key.map_id)
            .map(|p| p.profile.as_ref().map(|x| x.name.clone()));
        Some(ProfileView {
            map_dir_name: name,
            counting: stat.is_none_or(|s| s.estimated),
            key,
            chrono_entries,
            resolved,
            saved,
            presets: presets(),
            load,
            advice,
            picked,
        })
    })
}
