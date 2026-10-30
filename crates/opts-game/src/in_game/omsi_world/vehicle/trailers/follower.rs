//! One following unit of a trailer chain: its model, articulated segment, scripts and meshes.

use super::super::super::consist_plan::PlannedPart;
use super::*;

/// The three asset stores the unit's meshes are built into.
pub(super) type MeshAssets<'a> = (
    &'a mut Assets<Mesh>,
    &'a mut Assets<StandardMaterial>,
    &'a mut Assets<Image>,
);

/// What the chain collects while its units are spawned one after the other.
pub(super) struct ChainState {
    pub cams: CabCameras,
    pub driver_cams: CabCameras,
    /// Mass of the unit in front: what a trailer without a stated mass is assumed to weigh.
    pub previous_kg: f32,
    pub chain: Vec<Entity>,
    pub layout: Vec<super::super::super::rail::Part>,
    /// Script state: the head's engine, then the units with scripts of their own. A unit without
    /// `[scriptshare]` and with scripts runs an engine on its own entity; the others read the
    /// engine of the unit before them that owns a store (`owner`).
    pub owner: Entity,
    pub consist: Vec<opts_bevy::ConsistUnit>,
    /// Axles counted so far: trailer models number theirs after the head's (`Wheel_Rotation_2_L`).
    pub axle_base: usize,
    /// Rail: the head's mass (at least 20 t), then the stated mass of every following unit and
    /// whether it pulls; a unit that states none adds nothing.
    pub powered_kg: f32,
    pub total_kg: f32,
}

/// Reads the unit's model config: the file and its parsed content.
fn load_model(
    job: &WorldLoadJob,
    manifest: &BusManifest,
    dir: &Path,
) -> Result<(PathBuf, ModelConfig), String> {
    let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
    let cfg = job
        .install
        .find_in(dir, &cfg_rel)
        .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok((cfg, model))
}

/// Hands the model's text and script textures to the job, with fresh script canvases.
fn set_job_textures(job: &mut WorldLoadJob, model: &ModelConfig) {
    job.vehicle_text_defs = model.text_textures.clone();
    job.vehicle_script_defs = model.script_textures.clone();
    let canvases: Vec<opts_core::formats::script_texture::CpuScriptTexture> = model
        .script_textures
        .iter()
        .map(|st| opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height))
        .collect();
    job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
}

/// Y of the unit's rear axle in its own frame; `front_y` is the link to the unit in front.
fn rear_axle_y(manifest: &BusManifest, reversed: bool, front_y: f32) -> f32 {
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
    if rear_axle_y.is_finite() {
        rear_axle_y
    } else {
        front_y - 6.0
    }
}

/// The articulated segment of a unit: its joint to the part in front, limits, load and wheels.
fn articulated_segment(
    part: &PlannedPart,
    manifest: &BusManifest,
    load: &opts_sim::UnitLoad,
    axle_base: usize,
) -> opts_bevy::ArticulatedSegment {
    let reversed = part.turned;
    // The link to the part in front inside its unit (a unit's first file has none: only rail
    // places those, by `behind`, and the articulated segment is unused there).
    let (b, f) = part
        .joint
        .unwrap_or((Vec3::new(0.0, -part.behind, 0.0), Vec3::ZERO));
    let rear_axle_y = rear_axle_y(manifest, reversed, f.y);
    let max_alpha = manifest
        .coupling_front_limits
        .map_or(52.5, |l| l.max_alpha_deg);
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
}

/// The unit's load, and the running masses of the chain updated with it.
fn account_load(st: &mut ChainState, part: &PlannedPart, rail: bool) -> opts_sim::UnitLoad {
    let manifest = &part.manifest;
    // Road trailers assume the mass of the unit in front when they state none; rail cars
    // (bogies, joints and couplers are vehicles of their own and state 0) add what they state.
    let fallback_kg = if rail { 0.0 } else { st.previous_kg };
    if !rail && manifest_mass_kg(manifest).is_none() {
        tracing::warn!(trailer = %part.path.display(), assumed_kg = st.previous_kg, "trailer states no mass; the mass of the unit in front is used");
    }
    let load = opts_sim::UnitLoad::from_manifest_or(manifest, part.turned, fallback_kg);
    st.previous_kg = load.mass_kg;
    if rail {
        st.total_kg += load.mass_kg;
        if part.powered {
            st.powered_kg += load.mass_kg;
        }
    }
    load
}

/// The unit's own scripts, sounds, door points and passengers, and its place in the consist.
fn attach_unit_extras(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (root, seg): (Entity, Entity),
    st: &mut ChainState,
    part: &PlannedPart,
) {
    let (path, manifest, dir) = (&part.path, &part.manifest, &part.dir);
    // The unit's own sounds (an articulated bus can have its engine in the second unit); they
    // read the head's variables, the trailer shares its scripts.
    let mut engine_unit = None;
    if !manifest.script_share
        && super::super::scripts::attach_scripts(
            job,
            commands,
            seg,
            manifest,
            dir,
            super::super::scripts::script_seed(job.install.root(), path, 0),
            Some(st.chain.len()),
        )
    {
        st.owner = seg;
        engine_unit = Some(seg);
    }
    st.consist.push(opts_bevy::ConsistUnit {
        part: engine_unit,
        head: false,
        cables: super::super::scripts::cables_of(manifest),
        turned: part.turned,
    });
    super::super::doors::attach_unit_door_points(job, commands, root, seg, manifest, dir);
    crate::player_sound::attach_unit(job, commands, seg, Some(st.owner), manifest, dir);
    super::super::passengers::attach_unit(
        job,
        commands,
        root,
        seg,
        (engine_unit.is_some() || manifest.script_share).then_some(st.owner),
        manifest,
        dir,
    );
    st.axle_base += manifest.axles.len();
}

/// Builds the unit's meshes onto `seg`: the mesh and vertex counts.
fn attach_meshes(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    (meshes, materials, images): MeshAssets,
    (seg, owner): (Entity, Entity),
    lod0: &opts_core::formats::vehicles::model::ModelLod,
    (cfg, model, dir): (&Path, &ModelConfig, &Path),
) -> (usize, usize) {
    let model_dir =
        crate::in_game::omsi_world::mesh_dir::model_mesh_dir(&job.install, dir, cfg, model);
    let tex_dirs = vec![
        job.install
            .find_in(dir, "texture")
            .unwrap_or_else(|| dir.join("texture")),
        model_dir.join("texture"),
    ];
    job.cache.texture_replacements = repaint_map(job, model);
    let (count, verts, _, _) = load_segment_meshes_for(
        job,
        commands,
        (&mut *meshes, &mut *materials, &mut *images),
        (seg, owner),
        lod0,
        &model_dir,
        &tex_dirs,
    );
    job.cache.texture_replacements.clear();
    (count, verts)
}

/// Spawns one following unit behind the chain so far. `false` when it cannot be loaded and the
/// chain ends here.
pub(super) fn spawn_follower(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    assets: MeshAssets,
    root: Entity,
    (part, rail): (&PlannedPart, bool),
    st: &mut ChainState,
) -> bool {
    let (path, manifest, dir) = (&part.path, &part.manifest, &part.dir);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (cfg, model) = match load_model(job, manifest, dir) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), trailer = %path.display(), "cannot load trailer: {e}");
            return false;
        }
    };
    let Some(lod0) = model.detail_lod() else {
        return false;
    };
    set_job_textures(job, &model);
    let load = account_load(st, part, rail);
    let seg = commands
        .spawn((
            Name::new(name.clone()),
            Transform::default(),
            Visibility::default(),
            articulated_segment(part, manifest, &load, st.axle_base),
            InGameEntity,
        ))
        .id();
    st.chain.push(seg);
    attach_unit_extras(job, commands, (root, seg), st, part);
    let (count, verts) = attach_meshes(
        job,
        commands,
        assets,
        (seg, st.owner),
        &lod0,
        (&cfg, &model, dir),
    );
    tracing::info!(trailer = %name, meshes = count, vertices = verts, "trailer segment attached");
    st.layout.push(super::super::super::rail::Part {
        entity: seg,
        name,
        behind: part.behind,
        half_span: manifest.bogie_half_span.unwrap_or(0.0),
        reversed: part.turned,
        meshes: count,
        vertices: verts,
        body: manifest
            .bounding_box
            .map(|(size, centre)| super::super::super::BodyBox::from_bounding_box(size, centre)),
    });
    st.cams
        .extend(manifest.pax_cameras.iter().cloned().map(|c| (seg, c)));
    // A unit with a cab of its own: its driver cameras, for the cab switch.
    st.driver_cams
        .extend(manifest.driver_cameras.iter().cloned().map(|c| (seg, c)));
    true
}
