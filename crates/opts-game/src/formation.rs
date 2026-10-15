//! Train formations in the game: resolving a [`TrainFormation`] against the installation, and the
//! control-protocol methods `formation.validate` and `formation.spawn`.
//! Design: `docs/src/architecture/train-formation-editor.md`, ADR-0013.

pub mod store;

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::control::params::{opt_str, opt_u64};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_core::formats::trains::{
    FormationIssue, FormationSegment, FormationSource, Orientation, TrainFormation, VehicleInfo,
    VehicleLookup, VehicleRef,
};
use opts_core::formats::vehicles::BusManifest;
use serde_json::{json, Value};

use crate::in_game::omsi_world::{FormationCar, WorldLoadJob};
use crate::in_game::types::SimulationLoadState;
use crate::session_vehicles::{EntrySel, SessionVehicles, SpawnParams};
use store::{FormationStore, StoreError};

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("formation.validate", validate);
    registry.register("formation.spawn", spawn);
    registry.register("formation.save", save);
    registry.register("formation.list", list);
    registry.register("formation.delete", delete);
}

/// Where the user's formations are kept. Replaced in tests; the default is the user data folder.
#[derive(Resource, Debug, Clone)]
pub struct FormationLibrary(pub Option<FormationStore>);

impl Default for FormationLibrary {
    fn default() -> Self {
        Self(FormationStore::user())
    }
}

/// Looks vehicles up through the installation of the running session.
pub struct JobLookup<'a>(pub &'a WorldLoadJob);

impl JobLookup<'_> {
    fn manifest(path: &Path) -> Option<BusManifest> {
        BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()
    }
}

impl VehicleLookup for JobLookup<'_> {
    fn info(&self, file: &str) -> Option<VehicleInfo> {
        let path = self.0.bus_file(file)?;
        let head = Self::manifest(&path)?;
        let mut info = VehicleInfo::from_manifest(&head);
        // Follow the car's own `[couple_back]` chain as the spawner does: a link flagged
        // reversed leads back into the chain and ends it, with the last file turned around, so
        // the car's rear coupling is that file's front point.
        let (mut dir, mut last) = (path.parent()?.to_path_buf(), head);
        let mut ring_closed = false;
        for _ in 0..opts_core::formats::trains::MAX_PARTS {
            let Some(next) = last.couple_back.clone() else {
                break;
            };
            let Some(p) = self.0.find_vehicle_in(&dir, &next) else {
                break;
            };
            let Some(m) = Self::manifest(&p) else { break };
            info.parts += 1;
            ring_closed = last.couple_back_reversed;
            dir = p.parent()?.to_path_buf();
            last = m;
            if ring_closed {
                break;
            }
        }
        info.back_coupling = if ring_closed {
            last.coupling_front.is_some()
        } else {
            last.coupling_back.is_some()
        };
        Some(info)
    }
}

/// A validated formation as the spawner needs it: the head's file and the cars behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFormation {
    /// The first car (the player's vehicle).
    pub head: PathBuf,
    /// The cars behind it, in order.
    pub cars: Vec<FormationCar>,
}

/// Validates `formation` and resolves its files.
///
/// # Errors
/// Every [`FormationIssue`] found.
pub fn resolve(
    formation: &TrainFormation,
    job: &WorldLoadJob,
) -> Result<ResolvedFormation, Vec<FormationIssue>> {
    let issues = formation.validate(&JobLookup(job));
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut files = formation.segments.iter().filter_map(|s| match &s.vehicle {
        VehicleRef::File(f) => job
            .bus_file(f)
            .map(|p| (p, s.orientation == Orientation::Backward, s.powered)),
        VehicleRef::Group(_) => None,
    });
    let Some((head, _, _)) = files.next() else {
        return Err(vec![FormationIssue::TooShort]);
    };
    Ok(ResolvedFormation {
        head,
        cars: files
            .map(|(path, turned, powered)| FormationCar {
                path,
                turned,
                powered,
            })
            .collect(),
    })
}

/// Queues the spawn of a resolved formation at `entry` and returns the session vehicle's id.
pub fn queue_spawn(
    registry: &mut SessionVehicles,
    formation: &TrainFormation,
    resolved: &ResolvedFormation,
    entry: EntrySel,
) -> crate::session_vehicles::VehicleId {
    let stem = resolved
        .head
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    registry.request_spawn(SpawnParams {
        catalogue_id: stem,
        name: formation.name.clone(),
        bus_file: resolved.head.display().to_string(),
        repaint: String::new(),
        setvars: Vec::new(),
        entry,
        formation: Some(formation.clone()),
    })
}

fn session_up(world: &World) -> Result<(), RpcError> {
    if !world.contains_resource::<SessionVehicles>() {
        return Err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    }
    if world
        .get_resource::<SimulationLoadState>()
        .is_some_and(|s| !s.is_complete)
    {
        return Err(RpcError::new(
            code::NOT_READY,
            "the session is still loading",
        ));
    }
    Ok(())
}

fn orientation_of(text: Option<&str>) -> Result<Orientation, RpcError> {
    match text.map(str::to_ascii_lowercase).as_deref() {
        None | Some("forward" | "0") => Ok(Orientation::Forward),
        Some("backward" | "1") => Ok(Orientation::Backward),
        Some(o) => Err(RpcError::invalid_params(format!(
            "orientation `{o}` is not `forward` or `backward`"
        ))),
    }
}

/// The formation a call describes: a `.zug` of the installation (`zug`), or `cars`, an array of
/// vehicle files or `{vehicle, orientation}` objects. `reverse` turns the train around.
fn formation_from(world: &World, params: &Value) -> Result<TrainFormation, RpcError> {
    let mut formation = if let Some(id) = opt_str(params, "saved")? {
        let library = world.get_resource::<FormationLibrary>();
        let store = library
            .and_then(|l| l.0.as_ref())
            .ok_or_else(|| RpcError::new(code::NOT_READY, "no user data folder for formations"))?;
        store.read(id).map_err(store_error)?
    } else if let Some(zug) = opt_str(params, "zug")? {
        let job = world
            .get_resource::<WorldLoadJob>()
            .ok_or_else(|| RpcError::new(code::NOT_READY, "no session has been started"))?;
        let path = job.bus_file(zug).ok_or_else(|| {
            RpcError::new(
                code::NOT_FOUND,
                format!("`{zug}` is not in the installation"),
            )
        })?;
        let bytes = std::fs::read(&path)
            .map_err(|e| RpcError::new(code::NOT_FOUND, format!("{}: {e}", path.display())))?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        TrainFormation::from_zug(&bytes, &name, zug)
            .map_err(|e| RpcError::invalid_params(format!("{zug}: {e}")))?
            .0
    } else if let Some(cars) = params.get("cars") {
        let Value::Array(items) = cars else {
            return Err(RpcError::invalid_params("`cars` must be an array"));
        };
        let mut segments = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let (vehicle, orientation, powered) = match item {
                Value::String(s) => (s.as_str(), None, None),
                Value::Object(o) => (
                    o.get("vehicle").and_then(Value::as_str).ok_or_else(|| {
                        RpcError::invalid_params(format!("cars[{i}] needs a `vehicle`"))
                    })?,
                    o.get("orientation").and_then(Value::as_str),
                    o.get("powered").and_then(Value::as_bool),
                ),
                _ => {
                    return Err(RpcError::invalid_params(format!(
                        "cars[{i}] must be a string or an object"
                    )))
                }
            };
            segments.push(FormationSegment {
                vehicle: VehicleRef::from_entry(vehicle),
                orientation: orientation_of(orientation)?,
                powered,
            });
        }
        TrainFormation {
            name: opt_str(params, "name")?.unwrap_or("Formation").to_owned(),
            segments,
            source: FormationSource::Custom,
        }
    } else {
        return Err(RpcError::invalid_params("give `saved`, `zug` or `cars`"));
    };
    if params.get("reverse").and_then(Value::as_bool) == Some(true) {
        formation = formation.reversed();
    }
    Ok(formation)
}

fn issues_json(issues: &[FormationIssue]) -> Vec<Value> {
    issues
        .iter()
        .map(|i| json!({ "key": i.key(), "car": i.car(), "message": i.to_string() }))
        .collect()
}

fn validate(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        let formation = formation_from(world, params)?;
        let job = world
            .get_resource::<WorldLoadJob>()
            .ok_or_else(|| RpcError::new(code::NOT_READY, "no session has been started"))?;
        let issues = formation.validate(&JobLookup(job));
        Ok(json!({
            "ok": issues.is_empty(),
            "name": formation.name,
            "cars": formation.len(),
            "issues": issues_json(&issues),
        }))
    })();
    result.into()
}

fn spawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let formation = formation_from(world, params)?;
        let resolved = {
            let job = world
                .get_resource::<WorldLoadJob>()
                .ok_or_else(|| RpcError::new(code::NOT_READY, "no session has been started"))?;
            resolve(&formation, job)
        };
        let resolved = resolved.map_err(|issues| {
            RpcError::new(
                code::INVALID_PARAMS,
                issues
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        let entry = match (opt_u64(params, "entry_id")?, opt_str(params, "entry")?) {
            (Some(i), _) => EntrySel::Id(
                u32::try_from(i).map_err(|_| RpcError::invalid_params("`entry_id` too large"))?,
            ),
            (None, Some(n)) => EntrySel::Name(n.to_owned()),
            (None, None) => EntrySel::Default,
        };
        let id = queue_spawn(
            &mut world.resource_mut::<SessionVehicles>(),
            &formation,
            &resolved,
            entry,
        );
        Ok(json!({ "id": id, "queued": true, "cars": resolved.cars.len() + 1 }))
    })();
    result.into()
}

fn store_error(e: StoreError) -> RpcError {
    match e {
        StoreError::NotFound(_) => RpcError::new(code::NOT_FOUND, e.to_string()),
        StoreError::InvalidName(_) => RpcError::invalid_params(e.to_string()),
        _ => RpcError::new(code::FAILED, e.to_string()),
    }
}

fn library(world: &World) -> Result<FormationStore, RpcError> {
    world
        .get_resource::<FormationLibrary>()
        .and_then(|l| l.0.clone())
        .ok_or_else(|| RpcError::new(code::NOT_READY, "no user data folder for formations"))
}

/// `formation.save`: stores the formation the call describes under a new id made from its name
/// (or under `id`, replacing that file, when `overwrite` is true).
fn save(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        let mut formation = formation_from(world, params)?;
        if formation.is_empty() {
            return Err(RpcError::invalid_params("an empty formation is not saved"));
        }
        if let Some(name) = opt_str(params, "name")? {
            formation.name = name.to_owned();
        }
        let store = library(world)?;
        let id = match (
            opt_str(params, "id")?,
            params.get("overwrite").and_then(Value::as_bool) == Some(true),
        ) {
            (Some(id), true) => {
                store.write(id, &formation).map_err(store_error)?;
                id.to_owned()
            }
            (Some(_), false) => {
                return Err(RpcError::invalid_params("`id` needs `overwrite: true`"))
            }
            (None, _) => store.save_new(&formation).map_err(store_error)?,
        };
        Ok(json!({ "ok": true, "id": id, "name": formation.name, "cars": formation.len() }))
    })();
    result.into()
}

/// One row of `formation.list`.
fn saved_json(s: &store::SavedFormation) -> Value {
    let modified = s
        .modified
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    match &s.state {
        Ok(f) => json!({ "id": s.id, "name": f.name, "cars": f.len(), "modified": modified }),
        Err(e) => json!({ "id": s.id, "error": e.to_string(), "modified": modified }),
    }
}

fn list(world: &mut World, _: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        let store = library(world)?;
        let saved: Vec<Value> = store.list().iter().map(saved_json).collect();
        Ok(json!({ "dir": store.dir().display().to_string(), "formations": saved }))
    })();
    result.into()
}

fn delete(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        let id = opts_bevy::control::params::str_param(params, "id")?;
        library(world)?.delete(id).map_err(store_error)?;
        Ok(json!({ "ok": true, "id": id }))
    })();
    result.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from(params: Value) -> Result<TrainFormation, RpcError> {
        formation_from(&World::new(), &params)
    }

    #[test]
    fn cars_may_be_strings_or_objects_and_default_to_forward_unspecified_power() {
        let f = from(json!({
            "name": "Mine",
            "cars": [
                "vehicles\\A\\loco.ovh",
                {"vehicle": "vehicles\\A\\wagon.ovh", "orientation": "backward", "powered": false},
                {"vehicle": "Some Group"}
            ]
        }))
        .expect("parses");
        assert_eq!(f.name, "Mine");
        assert_eq!(f.source, FormationSource::Custom);
        assert_eq!(f.segments[0].orientation, Orientation::Forward);
        assert_eq!(f.segments[0].powered, None);
        assert_eq!(f.segments[1].orientation, Orientation::Backward);
        assert_eq!(f.segments[1].powered, Some(false));
        assert!(matches!(f.segments[2].vehicle, VehicleRef::Group(_)));
    }

    #[test]
    fn reverse_turns_the_train_around() {
        let f = from(
            json!({"cars": ["a.ovh", {"vehicle": "b.ovh", "orientation": "1"}], "reverse": true}),
        )
        .expect("parses");
        assert_eq!(f.segments[0].vehicle.as_str(), "b.ovh");
        assert_eq!(f.segments[0].orientation, Orientation::Forward);
        assert_eq!(f.segments[1].orientation, Orientation::Backward);
    }

    #[test]
    fn bad_parameters_are_reported_as_invalid_params() {
        for params in [
            json!({}),
            json!({"cars": "a.ovh"}),
            json!({"cars": [5]}),
            json!({"cars": [{"orientation": "forward"}]}),
            json!({"cars": [{"vehicle": "a.ovh", "orientation": "sideways"}]}),
        ] {
            let e = from(params.clone()).expect_err(&params.to_string());
            assert_eq!(e.code, code::INVALID_PARAMS, "{params}");
        }
    }

    #[test]
    fn a_zug_needs_a_running_session() {
        let e = from(json!({"zug": "Trains\\x.zug"})).expect_err("no session");
        assert_eq!(e.code, code::NOT_READY);
    }

    #[test]
    fn issues_are_listed_with_key_car_and_message() {
        let issues = [
            FormationIssue::TooShort,
            FormationIssue::NotRail { index: 2 },
        ];
        let v = issues_json(&issues);
        assert_eq!(v[0]["key"], "formation-err-too-short");
        assert!(v[0]["car"].is_null());
        assert_eq!(v[1]["car"], 2);
        assert!(v[1]["message"]
            .as_str()
            .is_some_and(|m| m.contains("car 3")));
    }
}
