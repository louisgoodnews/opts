//! The bus configuration itself: the checks that decide whether its choices can be applied, and the repaint and vehicle lookups.

use super::*;

impl BusConfiguration {
    /// The configuration of the choices in `setup` named `name`, stamped with `now`; `None` when
    /// no vehicle is chosen or it is not in the catalogue.
    #[must_use]
    pub fn from_setup(
        name: &str,
        catalogue: &GameCatalogue,
        setup: &NewGameSetup,
        now: i64,
    ) -> Option<Self> {
        if setup.no_vehicle {
            return None;
        }
        let vehicle = catalogue.find_vehicle(&setup.selected_vehicle)?;
        Some(Self::new(
            name,
            &vehicle.id,
            &setup.selected_livery,
            setup
                .custom_setvars
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            now,
        ))
    }

    /// A configuration with its options sorted; the id is given when it is added to a store.
    #[must_use]
    pub fn new(
        name: &str,
        vehicle: &str,
        repaint: &str,
        mut setvars: Vec<(String, i32)>,
        now: i64,
    ) -> Self {
        setvars.sort();
        Self {
            id: String::new(),
            name: name.trim().to_string(),
            vehicle: vehicle.to_string(),
            repaint: repaint.to_string(),
            setvars,
            notes: String::new(),
            favourite: false,
            created: now,
            last_used: now,
        }
    }

    /// Checks the configuration against the installation: `repaints` gives the repaints and
    /// declared variables of a catalogue vehicle (the wizard's own discovery, or a fixed answer
    /// in tests).
    #[must_use]
    pub fn check(
        &self,
        catalogue: &GameCatalogue,
        repaints: impl Fn(&VehicleItem) -> VehicleRepaints,
    ) -> Checked {
        let mut out = Checked {
            repaint: self.repaint.clone(),
            setvars: Vec::new(),
            problems: Vec::new(),
        };
        let Some(vehicle) = catalogue.vehicles.iter().find(|v| v.id == self.vehicle) else {
            out.problems.push(Problem::VehicleMissing {
                vehicle: self.vehicle.clone(),
            });
            return out;
        };
        let reps = repaints(vehicle);
        if !self.repaint.is_empty() && !reps.names.iter().any(|n| same_name(n, &self.repaint)) {
            out.problems.push(Problem::RepaintMissing {
                repaint: self.repaint.clone(),
            });
            out.repaint.clear();
        }
        let options = catalogue_of(&reps);
        for (name, value) in &self.setvars {
            match options.get(name) {
                None => out
                    .problems
                    .push(Problem::SetvarUnknown { name: name.clone() }),
                Some(e) if e.accepts(f64::from(*value)) => {
                    out.setvars.push((e.name.clone(), *value));
                }
                Some(_) => out.problems.push(Problem::SetvarRejected {
                    name: name.clone(),
                    value: *value,
                }),
            }
        }
        out
    }

    /// Writes the checked choices into `setup`: the vehicle, the repaint and the options (other
    /// fields keep their values). `None` (nothing written) when the vehicle is missing.
    pub fn apply_to(
        &self,
        catalogue: &GameCatalogue,
        checked: &Checked,
        setup: &mut NewGameSetup,
    ) -> Option<()> {
        let vehicle = catalogue.vehicles.iter().find(|v| v.id == self.vehicle)?;
        if !checked.usable() {
            return None;
        }
        setup.no_vehicle = false;
        setup.selected_vehicle = catalogue.vehicle_key(vehicle);
        setup.selected_livery = checked.repaint.clone();
        setup.custom_setvars = checked.setvars.iter().cloned().collect();
        Some(())
    }
}
