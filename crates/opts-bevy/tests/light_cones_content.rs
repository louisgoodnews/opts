//! Light cones of real content: a street lantern (a cone), a traffic signal and a tree (none),
//! and the headlights of two real vehicle models. Skipped without `OMSI_PATH`.

use std::path::PathBuf;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use opts_bevy::light_cones::{
    ConeKind, ConeLevel, LightCone, HEADLIGHT_CONE_LENGTH_M, HEADLIGHT_CORONA_BEAM_HALF_DEG,
};
use opts_bevy::scenery_runtime::{
    build_scenery_object, SceneryBuildAssets, SceneryLightAssets, SceneryObjectInput,
};
use opts_bevy::{spawn_vehicle_glows, NoMapLighting};
use opts_core::formats::scenery::SceneryObject;
use opts_core::formats::vehicles::model::ModelConfig;

fn omsi() -> Option<PathBuf> {
    let root = std::env::var_os("OMSI_PATH").map(PathBuf::from);
    if root.is_none() {
        eprintln!("OMSI_PATH not set; skipping");
    }
    root
}

/// Builds the scenery object at `path` (no meshes) and returns its cones and whether the root
/// carries the `[nomaplighting]` marker.
fn cones_of_sco(path: &std::path::Path) -> Option<(Vec<LightCone>, bool)> {
    let bytes = std::fs::read(path).ok()?;
    let sco = SceneryObject::parse(&bytes).expect("sco parses");
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            watch_for_changes_override: Some(false),
            ..Default::default()
        },
        bevy::transform::TransformPlugin,
    ))
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>();
    let root = app
        .world_mut()
        .spawn((Transform::default(), Visibility::Inherited))
        .id();
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>| {
                let input = SceneryObjectInput {
                    sco: &sco,
                    mesh_parts: sco.meshes.iter().map(|_| Vec::new()).collect(),
                    scripts: None,
                    signal: None,
                };
                let mut lights = SceneryLightAssets::default();
                let mut assets = SceneryBuildAssets {
                    meshes: &mut meshes,
                    materials: &mut materials,
                    lights: &mut lights,
                };
                build_scenery_object(&mut commands, root, input, &mut assets);
            },
        )
        .expect("build");
    let cones = app
        .world_mut()
        .query::<&LightCone>()
        .iter(app.world())
        .cloned()
        .collect();
    let marked = app.world().get::<NoMapLighting>(root).is_some();
    Some((cones, marked))
}

#[test]
fn a_street_lantern_gets_one_cone_and_signals_and_trees_none() {
    let Some(root) = omsi() else {
        return;
    };
    let dir = root.join("Sceneryobjects");
    let Some((lantern, marked)) =
        cones_of_sco(&dir.join("Streetobjects_RUE/neonlight_M_whip_S.sco"))
    else {
        return;
    };
    assert_eq!(lantern.len(), 1, "one `cone` light");
    let c = &lantern[0];
    assert_eq!(c.kind, ConeKind::Lantern);
    assert!(matches!(c.level, ConeLevel::Glow(_)));
    // Pointing down from the lamp head, so the cone reaches towards the ground; 200 degrees
    // outer is held at the 40 degree half angle.
    assert!(c.shape.dir.y < -0.9, "{:?}", c.shape.dir);
    assert!((c.shape.half_angle - 40f32.to_radians()).abs() < 1e-5);
    assert!(c.shape.length >= 2.0 && c.shape.length <= 14.0);
    assert!(c.shape.apex.y > 5.0, "lamp head height {}", c.shape.apex.y);
    // The file also has `[nomaplighting]`.
    assert!(marked, "the root of a [nomaplighting] object is marked");

    if let Some((signal, _)) = cones_of_sco(&dir.join("Verkehrszeichen_MC/Ampel_KfzL_1.sco")) {
        assert!(
            signal.is_empty(),
            "a signal head has lights but no cone flag"
        );
    }
    if let Some((tree, marked)) = cones_of_sco(&dir.join("Trees_MC/arbor_1.sco")) {
        assert!(tree.is_empty());
        assert!(!marked);
    }
}

fn headlight_cones_of_model(path: &std::path::Path) -> Option<(usize, usize, Vec<LightCone>)> {
    let bytes = std::fs::read(path).ok()?;
    let model = ModelConfig::parse_bytes(&bytes).expect("model.cfg parses");
    let headlights = opts_bevy::vehicle_glow::detail_lights(&model)
        .filter(|l| opts_bevy::vehicle_glow::is_headlight(l))
        .count();
    let glows = opts_bevy::vehicle_glow::detail_lights(&model).count();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin));
    let root = app.world_mut().spawn(Transform::default()).id();
    {
        let mut commands = app.world_mut().commands();
        spawn_vehicle_glows(&mut commands, root, root, &model);
    }
    app.world_mut().flush();
    let cones = app
        .world_mut()
        .query::<&LightCone>()
        .iter(app.world())
        .cloned()
        .collect();
    Some((glows, headlights, cones))
}

#[test]
fn real_vehicle_headlights_get_cones_and_taillights_do_not() {
    let Some(root) = omsi() else {
        return;
    };
    for rel in [
        "Vehicles/VW_Golf_2/model/model.cfg",
        "Vehicles/MAN_F90/model/model.cfg",
    ] {
        let Some((glows, headlights, cones)) = headlight_cones_of_model(&root.join(rel)) else {
            eprintln!("{rel}: not installed; skipping");
            continue;
        };
        eprintln!(
            "{rel}: {glows} light effects, {headlights} headlights, {} cones",
            cones.len()
        );
        assert!(headlights >= 2, "{rel}: a headlight pair");
        assert_eq!(cones.len(), headlights, "{rel}: one cone per headlight");
        assert!(
            cones.len() < glows,
            "{rel}: tail and indicator lights have no cone"
        );
        for c in &cones {
            assert_eq!(c.kind, ConeKind::Headlight);
            // Forward is Bevy -Z; an outer angle of 190 degrees is the angle the corona is seen
            // from, so the haze is the narrow low beam (2026-10-22), not the widest cone.
            assert!(c.shape.dir.z < -0.9, "{rel}: {:?}", c.shape.dir);
            assert!(
                (c.shape.half_angle - HEADLIGHT_CORONA_BEAM_HALF_DEG.to_radians()).abs() < 1e-5,
                "{rel}: half angle {} degrees",
                c.shape.half_angle.to_degrees()
            );
            assert_eq!(c.shape.length, HEADLIGHT_CONE_LENGTH_M);
        }
    }
}
