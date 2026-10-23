//! Trailer chains and consist masses.

use super::segments::{load_segment_meshes_for, repaint_map};
use super::*;

/// One car of a train formation behind the head: the vehicle file and whether it is turned
/// around (see `opts_core::formats::trains`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationCar {
    /// The resolved vehicle file.
    pub path: PathBuf,
    /// The car runs turned around in the train.
    pub turned: bool,
    /// Whether the car pulls: `None` lets the loader decide (a car made of several files does).
    pub powered: Option<bool>,
}

/// The most vehicle files one spawn follows: a single vehicle's `[couple_back]` chain, or a
/// formation's cars with their own chains.
pub(super) fn max_parts(formation: bool) -> usize {
    if formation {
        opts_core::formats::trains::MAX_PARTS
    } else {
        40
    }
}

/// The two coupling points that join a unit to the one in front of it, in the frame of the
/// train (y forward): the rear point of the unit in front and the front point of the unit behind,
/// each relative to its own vehicle's origin. A turned unit has its points negated and swapped
/// (its model's front point faces the rear of the train). `None` when a point is missing.
pub(in super::super::super) fn coupling_joint(
    parent: &BusManifest,
    parent_turned: bool,
    own: &BusManifest,
    own_turned: bool,
) -> Option<(Vec3, Vec3)> {
    let rear_of_parent = if parent_turned {
        parent.coupling_front.map(|c| -c)
    } else {
        parent.coupling_back
    }?;
    let front_of_own = if own_turned {
        own.coupling_back.map(|c| -c)
    } else {
        own.coupling_front
    }?;
    Some((rear_of_parent, front_of_own))
}

/// The installation as the planner reads it.
pub(super) struct JobFiles<'a>(&'a WorldLoadJob);

impl super::super::consist_plan::Files for JobFiles<'_> {
    fn find_in(&self, base: &Path, rel: &str) -> Option<PathBuf> {
        self.0.install.find_in(base, rel)
    }

    fn manifest(&self, path: &Path) -> Option<BusManifest> {
        BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()
    }
}

/// Spawns the units behind the head `root`: its `[couple_back]` chain, then the cars of a train
/// formation (`job.formation_cars`), each with its own chain, where `consist_plan` puts them.
/// Camera presets with the unit entity they are defined in.
pub(in super::super::super) type CabCameras =
    Vec<(Entity, opts_core::formats::vehicles::bus::BusCamera)>;

/// Returns the passenger and the driver camera presets of all trailers with the segment entity
/// they are defined in.
#[allow(clippy::too_many_arguments)]
pub(in super::super::super) fn spawn_trailer_chain(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    root: Entity,
    bus_dir: &Path,
    tractor: &BusManifest,
) -> (CabCameras, CabCameras) {
    let formation: Vec<FormationCar> = std::mem::take(&mut job.formation_cars);
    let rail = super::super::is_rail(tractor);
    let plan = super::super::consist_plan::plan(
        bus_dir,
        tractor,
        &formation,
        max_parts(!formation.is_empty()),
        &JobFiles(job),
    );
    let mut cams = Vec::new();
    let mut driver_cams = Vec::new();
    // Mass of the unit in front: what a trailer without a stated mass is assumed to weigh.
    let mut previous_kg = manifest_mass_kg(tractor).unwrap_or(HEAD_MASS_DEFAULT_KG);
    let mut chain = Vec::new();
    let mut layout = Vec::new();
    // Script state: the head's engine, then the units with scripts of their own. A unit without
    // `[scriptshare]` and with scripts runs an engine on its own entity; the others read the
    // engine of the unit before them that owns a store (`owner`).
    let mut owner = root;
    let mut consist = vec![opts_bevy::ConsistUnit {
        part: None,
        head: true,
        cables: super::scripts::cables_of(tractor),
        turned: false,
    }];
    // Axles counted so far: trailer models number theirs after the head's (`Wheel_Rotation_2_L`).
    let mut axle_base = tractor.axles.len();
    // Rail: the head's mass (at least 20 t), then the stated mass of every following unit and
    // whether it pulls; a unit that states none adds nothing.
    let head_kg = commands_mass(tractor);
    let mut powered_kg = head_kg;
    let mut total_kg = head_kg;
    for part in &plan.followers {
        let (path, manifest, dir) = (&part.path, &part.manifest, &part.dir);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let loaded = (|| -> Result<_, String> {
            let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
            let cfg = job
                .install
                .find_in(dir, &cfg_rel)
                .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
            let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            Ok((cfg, model))
        })();
        let (cfg, model) = match loaded {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(at = %opts_bevy::position_tag(), trailer = %path.display(), "cannot load trailer: {e}");
                break;
            }
        };
        let Some(detail) = model.detail_lod() else {
            break;
        };
        let lod0 = &detail;
        job.vehicle_text_defs = model.text_textures.clone();
        job.vehicle_script_defs = model.script_textures.clone();
        let canvases: Vec<opts_core::formats::script_texture::CpuScriptTexture> = model
            .script_textures
            .iter()
            .map(|st| {
                opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height)
            })
            .collect();
        job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
        let reversed = part.turned;
        // The link to the part in front inside its unit (a unit's first file has none: only rail
        // places those, by `behind`, and the articulated segment is unused there).
        let (b, f) = part
            .joint
            .unwrap_or((Vec3::new(0.0, -part.behind, 0.0), Vec3::ZERO));
        let rear_axle_y = if reversed {
            // Mirrored: the model's frontmost axle becomes the rearmost.
            -manifest
                .axles
                .iter()
                .map(|a| a.longitudinal_m)
                .fold(f32::NEG_INFINITY, f32::max)
        } else {
            manifest
                .axles
                .iter()
                .map(|a| a.longitudinal_m)
                .fold(f32::INFINITY, f32::min)
        };
        let rear_axle_y = if rear_axle_y.is_finite() {
            rear_axle_y
        } else {
            f.y - 6.0
        };
        let max_alpha = manifest
            .coupling_front_limits
            .map_or(52.5, |l| l.max_alpha_deg);
        // Road trailers assume the mass of the unit in front when they state none; rail cars
        // (bogies, joints and couplers are vehicles of their own and state 0) add what they state.
        let fallback_kg = if rail { 0.0 } else { previous_kg };
        if !rail && manifest_mass_kg(manifest).is_none() {
            tracing::warn!(trailer = %path.display(), assumed_kg = previous_kg, "trailer states no mass; the mass of the unit in front is used");
        }
        let load = opts_sim::UnitLoad::from_manifest_or(manifest, reversed, fallback_kg);
        previous_kg = load.mass_kg;
        if rail {
            total_kg += load.mass_kg;
            if part.powered {
                powered_kg += load.mass_kg;
            }
        }
        let seg = commands
            .spawn((
                Name::new(name.clone()),
                Transform::default(),
                Visibility::default(),
                {
                    let mut s = opts_bevy::ArticulatedSegment::new(
                        Vec2::new(b.x, b.y),
                        Vec2::new(f.x, f.y),
                        rear_axle_y,
                        max_alpha,
                    );
                    s.reversed = reversed;
                    s.load = Some(load.clone());
                    // The bus files list min and max in either order (3 files have 20,-20).
                    s.beta_limits_deg = manifest.coupling_front_limits.map(|l| {
                        (
                            l.min_beta_deg.min(l.max_beta_deg),
                            l.min_beta_deg.max(l.max_beta_deg),
                        )
                    });
                    s.set_wheels(
                        axle_base,
                        manifest
                            .axles
                            .iter()
                            .map(|a| a.wheel_diameter_m.filter(|&d| d > 0.1).unwrap_or(0.98) * 0.5),
                    );
                    s
                },
                InGameEntity,
            ))
            .id();
        chain.push(seg);
        // The unit's own sounds (an articulated bus can have its engine in the second unit); they
        // read the head's variables, the trailer shares its scripts.
        let mut engine_unit = None;
        if !manifest.script_share
            && super::scripts::attach_scripts(
                job,
                commands,
                seg,
                manifest,
                dir,
                super::scripts::script_seed(job.install.root(), path, 0),
                Some(chain.len()),
            )
        {
            owner = seg;
            engine_unit = Some(seg);
        }
        consist.push(opts_bevy::ConsistUnit {
            part: engine_unit,
            head: false,
            cables: super::scripts::cables_of(manifest),
            turned: part.turned,
        });
        super::doors::attach_unit_door_points(job, commands, root, seg, manifest, dir);
        crate::player_sound::attach_unit(job, commands, seg, Some(owner), manifest, dir);
        super::passengers::attach_unit(
            job,
            commands,
            root,
            seg,
            (engine_unit.is_some() || manifest.script_share).then_some(owner),
            manifest,
            dir,
        );
        axle_base += manifest.axles.len();
        let model_dir = cfg.parent().map(Path::to_path_buf).unwrap_or_default();
        let tex_dirs = vec![
            job.install
                .find_in(dir, "texture")
                .unwrap_or_else(|| dir.join("texture")),
            model_dir.join("texture"),
        ];
        job.cache.texture_replacements = repaint_map(job, &model);
        let (count, verts, _, _) = load_segment_meshes_for(
            job, commands, meshes, materials, images, seg, owner, lod0, &model_dir, &tex_dirs,
        );
        job.cache.texture_replacements.clear();
        tracing::info!(trailer = %name, meshes = count, vertices = verts, "trailer segment attached");
        layout.push(super::super::rail::Part {
            entity: seg,
            name,
            behind: part.behind,
            half_span: manifest.bogie_half_span.unwrap_or(0.0),
            reversed,
            meshes: count,
            vertices: verts,
            body: manifest
                .bounding_box
                .map(|(size, centre)| super::super::BodyBox::from_bounding_box(size, centre)),
        });
        cams.extend(manifest.pax_cameras.iter().cloned().map(|c| (seg, c)));
        // A unit with a cab of its own: its driver cameras, for the cab switch.
        driver_cams.extend(manifest.driver_cameras.iter().cloned().map(|c| (seg, c)));
    }
    if consist.iter().any(|u| u.part.is_some()) || consist.iter().any(|u| !u.cables.is_empty()) {
        commands
            .entity(root)
            .insert(opts_bevy::ScriptConsist { units: consist });
    }
    if rail {
        // The whole consist's masses for the dynamics (set once the head has its `RailPlayer`).
        let load = opts_sim::rail::ConsistLoad {
            total_kg,
            powered_kg,
            braked_kg: total_kg,
        };
        commands.queue(move |world: &mut World| {
            if let Some(mut player) = world.get_mut::<super::super::rail::RailPlayer>(root) {
                player.set_consist(load);
            }
        });
    }
    if !chain.is_empty() {
        if !rail {
            commands
                .entity(root)
                .insert(opts_bevy::ArticulatedChain { segments: chain });
        }
        commands
            .entity(root)
            .insert((super::super::rail::VehicleComposition {
                head: super::super::rail::Part {
                    entity: root,
                    name: tractor
                        .friendly_name
                        .as_ref()
                        .map(|f| f.model.clone())
                        .unwrap_or_default(),
                    behind: 0.0,
                    half_span: tractor.bogie_half_span.unwrap_or(0.0),
                    reversed: false,
                    meshes: 0,
                    vertices: 0,
                    body: tractor.bounding_box.map(|(size, centre)| {
                        super::super::BodyBox::from_bounding_box(size, centre)
                    }),
                },
                followers: layout,
            },));
    }
    (cams, driver_cams)
}

/// Mass the simulation assumes for a head unit that states none (kg).
const HEAD_MASS_DEFAULT_KG: f32 = 11_500.0;

/// The mass a vehicle file states (kg); `None` when it states none or an implausible one.
pub(super) fn manifest_mass_kg(manifest: &BusManifest) -> Option<f32> {
    manifest
        .mass_tonnes
        .filter(|&m| m > 0.5)
        .map(|m| m * 1_000.0)
}

/// What a following unit adds to the consist: its stated mass, else `previous_kg` (the unit in
/// front; the installed consists have a median trailer/head ratio of 0.90, `consist_census`), and
/// its axles with those marked driven (`achse_antrieb`).
#[cfg(test)]
pub(super) fn unit_load(
    manifest: &BusManifest,
    reversed: bool,
    previous_kg: f32,
) -> opts_sim::UnitLoad {
    opts_sim::UnitLoad::from_manifest_or(manifest, reversed, previous_kg)
}

/// Mass of a rail consist for the dynamics (kg): the head unit's mass, at least 20 t (the other
/// units usually state 0; assumed, see `docs/src/simulation/rail-vehicles.md`).
pub(super) fn commands_mass(manifest: &BusManifest) -> f32 {
    (manifest.mass_tonnes.unwrap_or(0.0) * 1000.0).max(20_000.0)
}
