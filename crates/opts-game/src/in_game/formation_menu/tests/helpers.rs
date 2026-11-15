//! Shared set-up of the editor tests: the app, the editor's state and the store directory.

use super::*;

pub(super) fn store_dir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let d = std::env::var_os("HOME")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
        .join(".local/share/opts/benchmark/formation/editor-tests")
        .join(format!(
            "{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

pub(super) fn app() -> App {
    let mut app = App::new();
    app.add_message::<ButtonClicked>()
        .insert_resource(FormationLibrary(Some(FormationStore::new(store_dir()))))
        .init_resource::<FormationEditor>()
        .init_resource::<FormationCommands>()
        .init_resource::<VehicleMenu>()
        .init_resource::<SessionVehicles>()
        .add_systems(Update, actions::formation_actions);
    {
        let mut ed = app.world_mut().resource_mut::<FormationEditor>();
        for (rel, label) in [
            ("Vehicles/BR218/BR218_OR.ovh", "BR218 / BR218_OR  (80 t)"),
            ("Vehicles/Bm238/By438_mi.ovh", "Bm238 / By438_mi  (40 t)"),
            ("Vehicles/Gueterwagen/Zas.ovh", "Gueterwagen / Zas  (40 t)"),
        ] {
            ed.cars.push(Car {
                rel: rel.into(),
                label: label.into(),
                mass_t: Some(40.0),
            });
        }
        ed.presets = vec![
            "Trains/regio_grundorf.zug".into(),
            "Trains/X10_ICE1.zug".into(),
        ];
    }
    app
}

pub(super) fn say(app: &mut App, lines: &[&str]) {
    app.world_mut()
        .resource_mut::<FormationCommands>()
        .0
        .extend(lines.iter().map(|l| (*l).to_owned()));
    app.update();
}

pub(super) fn ed(app: &App) -> &FormationEditor {
    app.world().resource::<FormationEditor>()
}

pub(super) fn files(app: &App) -> Vec<String> {
    ed(app)
        .formation()
        .segments
        .iter()
        .map(|s| {
            s.vehicle
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}
