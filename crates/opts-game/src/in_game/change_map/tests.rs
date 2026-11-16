//! Tests of the carry-over rules of a map change.

use super::*;
use crate::session_vehicles::{Lifecycle, SessionVehicles, SpawnParams};

fn map(id: &str, friendly: &str, entries: &[&str]) -> MapItem {
    MapItem {
        id: id.into(),
        name: id.into(),
        friendly_name: friendly.into(),
        description: String::new(),
        tile_count: 0,
        standard_depot: String::new(),
        years: String::new(),
        entry_points: entries.iter().map(|e| (*e).to_owned()).collect(),
    }
}

fn catalogue() -> GameCatalogue {
    GameCatalogue {
        maps: vec![
            map("Berlin-Spandau", "Berlin Spandau", &["Depot", "Rathaus"]),
            map("Grundorf", "Grundorf", &["Dorfplatz"]),
            map("Grundorf Nord", "Grundorf Nord", &[]),
        ],
        vehicles: vec![
            VehicleItem::named("MAN_SD77", "MAN", "SD77"),
            VehicleItem::named("Citaro", "Mercedes", "Citaro"),
        ],
    }
}

fn record(catalogue_id: &str, name: &str, repaint: &str) -> SessionVehicleRecord {
    let mut registry = SessionVehicles::default();
    let id = registry.request_spawn(SpawnParams {
        catalogue_id: catalogue_id.into(),
        name: name.into(),
        bus_file: String::new(),
        repaint: repaint.into(),
        setvars: vec![("doors".into(), 1)],
        entry: crate::session_vehicles::EntrySel::Default,
        formation: None,
    });
    let mut r = registry.get(id).expect("record").clone();
    r.lifecycle = Lifecycle::Alive;
    r
}

fn inputs<'a>(cat: &'a GameCatalogue, active: Option<&'a SessionVehicleRecord>) -> PlanInputs<'a> {
    PlanInputs {
        catalogue: cat,
        active,
        others: vec!["Citaro".into()],
        clock: (
            CalendarDate::new(2026, 3, 14),
            8.0 * 3600.0 + 45.0 * 60.0,
            "Rain".into(),
        ),
    }
}

mod carry_over;
mod describe;
mod lookup;
