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
        crate::passengers::PassengerRig::from_files(&cabin, &paths, ticket_pack(job), seed)
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
