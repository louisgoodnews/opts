//! Actions on the chosen map's profiles: apply a choice, create a profile from the current settings, delete one.

use super::*;

/// *Apply Selected*: the choice is used for the next drive of this map; with `make_default` it is
/// also written as the profile of this key (`default.toml` or the Chrono entry's file).
///
/// # Errors
/// No map, no such profile, or the file cannot be written.
pub fn apply(world: &mut World, choice: &Choice, make_default: bool) -> Result<(), ServiceError> {
    let v = view(world).ok_or(ServiceError::NoMap)?;
    let profile = find(&v, choice)?;
    if make_default {
        if let Some(p) = &profile {
            let store = world
                .resource::<ProfileStoreRes>()
                .0
                .clone()
                .ok_or(ServiceError::NoStore)?;
            store
                .save(&v.key, p)
                .map_err(|e| ServiceError::Io(e.to_string()))?;
        }
    }
    world.resource_mut::<MapProfilePick>().0 = Some(PickedProfile {
        map: v.map_dir_name,
        profile,
    });
    Ok(())
}

/// *Create New from Current*: a profile of the current global values of the map-eligible
/// settings, saved for the key (`for_chrono` false: the map's default). Returns its name.
///
/// # Errors
/// No map, no folder, or the file cannot be written.
pub fn create_from_current(
    world: &mut World,
    name: &str,
    for_chrono: bool,
) -> Result<String, ServiceError> {
    let v = view(world).ok_or(ServiceError::NoMap)?;
    let store = world
        .resource::<ProfileStoreRes>()
        .0
        .clone()
        .ok_or(ServiceError::NoStore)?;
    let profile = {
        let registry = world.resource::<SettingsRegistry>();
        let settings = world.resource::<GameSettings>();
        // A key nothing is stored for holds its default: the profile pins the value the
        // person sees now, not "whatever the global setting is later".
        MapProfile::from_current(name, registry, |k| {
            settings
                .raw(k)
                .or_else(|| registry.get(k).map(|d| d.default.to_string()))
        })
    };
    let key = if for_chrono && v.key.chrono_id != DEFAULT_CHRONO {
        v.key.clone()
    } else {
        v.key.map_default()
    };
    store
        .save(&key, &profile)
        .map_err(|e| ServiceError::Io(e.to_string()))?;
    Ok(profile.name)
}

/// *Delete*: removes the profile file of the key (the Chrono one when `for_chrono` and one is
/// active, else the map's default). Returns whether there was one.
///
/// # Errors
/// No map, no folder, or the file cannot be removed.
pub fn delete(world: &mut World, for_chrono: bool) -> Result<bool, ServiceError> {
    let v = view(world).ok_or(ServiceError::NoMap)?;
    let store = world
        .resource::<ProfileStoreRes>()
        .0
        .clone()
        .ok_or(ServiceError::NoStore)?;
    let key = if for_chrono && v.key.chrono_id != DEFAULT_CHRONO {
        v.key.clone()
    } else {
        v.key.map_default()
    };
    store
        .delete(&key)
        .map_err(|e| ServiceError::Io(e.to_string()))
}
