//! The vehicle's fleet number: the predefined string `number` (read by 4,189 script lines: the
//! ATRON and IBIS printers' vehicle number, plates, displays). OMSI takes it from the `.org` list
//! the `.bus` names (`[number]`, one number per line, a number listed more often drawn more
//! often); the player can choose one, the default is one of the list. OPTS draws it from the
//! vehicle's script seed, so a vehicle keeps its number from run to run.

use std::path::Path;

use opts_bevy::install::OmsiInstall;
use opts_core::formats::org::RegistrationPool;
use opts_core::formats::vehicles::BusManifest;

/// The fleet number of a vehicle whose `.bus` is in `bus_dir`, drawn with `seed`; `None` when
/// the `.bus` names no `.org` list or it cannot be read.
#[must_use]
pub(super) fn fleet_number(
    install: &OmsiInstall,
    bus_dir: &Path,
    manifest: &BusManifest,
    seed: u64,
) -> Option<String> {
    let file = manifest.number_file.as_deref()?.trim();
    let path = install.find_in(bus_dir, file)?;
    let pool = RegistrationPool::parse_bytes(&std::fs::read(path).ok()?).ok()?;
    pick(&pool, seed)
}

/// The entry of `pool` that `seed` draws (weighted by how often an entry is listed).
fn pick(pool: &RegistrationPool, seed: u64) -> Option<String> {
    let n = pool.total_count();
    (n > 0)
        .then(|| pool.select_by_index((seed % n as u64) as usize))
        .flatten()
        .map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_draws_a_number_of_the_list() {
        let pool = RegistrationPool::parse("9210\r\n9338\r\n").expect("pool");
        assert_eq!(pick(&pool, 0).as_deref(), Some("9210"));
        assert_eq!(pick(&pool, 7).as_deref(), Some("9338"));
        assert_eq!(
            pick(&pool, 7),
            pick(&pool, 7),
            "the same seed, the same number"
        );
    }
}
