//! The passenger rig of the player's vehicle (M-0101): its cabin, its paths and the map's ticket
//! system and currency.

use opts_core::formats::money::MoneySystem;
use opts_core::formats::ticketpack::TicketPack;

use super::*;

/// The map's ticket pack (`[ticketpack]`), or the built-in one when it is not installed (a map
/// may name a pack that is not part of the installation).
pub(super) fn ticket_pack(job: &WorldLoadJob) -> TicketPack {
    let named = job.manifest.ticket_pack.as_deref();
    match named
        .and_then(|rel| job.install.find_file(rel))
        .and_then(|p| std::fs::read(p).ok())
    {
        Some(bytes) => TicketPack::parse_bytes(&bytes),
        None => {
            tracing::warn!(
                ticketpack = ?named,
                "the map's ticket pack is not installed: the built-in tickets are used"
            );
            TicketPack::fallback()
        }
    }
}

/// The map's currency (`[moneysystem]`), when it is installed.
pub(super) fn money_system(job: &WorldLoadJob) -> Option<MoneySystem> {
    job.manifest
        .money_system
        .as_deref()
        .and_then(|rel| job.install.find_file(rel))
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| MoneySystem::parse_bytes(&b).ok())
}

/// Gives the player's vehicle its passengers when it has a cabin with an entry and a paths file.
pub(super) fn attach_passengers(
    job: &WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    manifest: &BusManifest,
    bus_dir: &Path,
    seed: u64,
) {
    let read = |rel: Option<&str>| {
        rel.and_then(|r| job.install.find_in(bus_dir, r))
            .and_then(|p| std::fs::read(p).ok())
    };
    let (Some(cabin), Some(paths)) = (
        read(manifest.passengercabin_cfg.as_deref()),
        read(manifest.paths_cfg.as_deref()),
    ) else {
        return;
    };
    let Some(rig) =
        crate::passengers::PassengerRig::from_files(&cabin, &paths, ticket_pack(job), seed, root)
    else {
        return;
    };
    tracing::info!(
        places = rig.sim.cabin.places.len(),
        entries = rig.sim.cabin.entries.len(),
        tickets = rig.sim.tickets.tickets.len(),
        "passengers: cabin loaded"
    );
    commands.entity(root).insert(rig);
}

/// A following unit of the chain joins the head's passengers: its cabin and paths are added to
/// the rig once both exist (the unit's own frame is `seg`).
pub(super) fn attach_unit(
    job: &WorldLoadJob,
    commands: &mut Commands,
    head: Entity,
    seg: Entity,
    store: Option<Entity>,
    manifest: &BusManifest,
    dir: &Path,
) {
    let read = |rel: Option<&str>| {
        rel.and_then(|r| job.install.find_in(dir, r))
            .and_then(|p| std::fs::read(p).ok())
    };
    let (Some(cabin), Some(paths)) = (
        read(manifest.passengercabin_cfg.as_deref()),
        read(manifest.paths_cfg.as_deref()),
    ) else {
        return;
    };
    let Ok(paths) = opts_core::formats::vehicles::PassengerPaths::parse_bytes(&paths) else {
        return;
    };
    let mut unit = crate::passengers::unit_spec(
        &opts_core::formats::vehicles::cabin::PassengerCabin::parse_bytes(&cabin),
        &paths,
    );
    // The doors report in the engine of the unit's scripts: its own (the NF6D's cars B and C),
    // or the one it shares (`[scriptshare]`: the articulated bus's second unit). A unit with
    // neither has nobody to open its doors: they are left out (places and paths stay).
    let store = store.unwrap_or_else(|| {
        unit.cabin.entries.clear();
        unit.cabin.exits.clear();
        unit.cabin.entry_no_sale.clear();
        head
    });
    tracing::info!(
        unit = %dir.display(),
        places = unit.cabin.places.len(),
        entries = unit.cabin.entries.len(),
        points = unit.paths.points.len(),
        link_to_prev = ?unit.link_to_prev,
        link_to_next = ?unit.link_to_next,
        "passengers: unit joins the chain"
    );
    commands.queue(move |world: &mut World| {
        if let Some(mut rig) = world.get_mut::<crate::passengers::PassengerRig>(head) {
            rig.add_unit(seg, unit, store);
        }
    });
}
