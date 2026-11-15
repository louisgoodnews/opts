//! The whole chain on real meshes: loaded panes become mirror cameras on the player vehicle only.

use super::*;

use super::support::{build_mirror_test_app, load_mirror_parts, spawn_test_vehicle};

/// The whole chain on real meshes: the loader registers the pane materials, and the mirror
/// system turns the panes of a player vehicle into cameras (one per `reflexion<N>` material).
#[test]
fn loaded_mirror_parts_become_mirror_cameras_on_the_player_vehicle_only() {
    use opts_bevy::mirrors::MirrorCamera;
    use opts_bevy::OmsiInstall;

    let Some(root) = root() else { return };
    let dir = root.join("Vehicles/MAN_NewLionsCity/model/generic");
    if !dir.join("Spiegel_links.o3d").exists() {
        eprintln!("MAN_NewLionsCity not installed; skipping");
        return;
    }
    let install = OmsiInstall::new(&root);
    let mut app = build_mirror_test_app();
    let parts = load_mirror_parts(&mut app, &install, &dir);

    let panes = parts
        .iter()
        .filter(|p| opts_bevy::glass::surface_kind(p.material.id()) == Some(SurfaceKind::Mirror))
        .count();
    assert!(
        panes >= 3,
        "pane parts registered by the loader: {panes} of {}",
        parts.len()
    );

    spawn_test_vehicle(&mut app, &parts, false);
    app.update();
    app.update();
    let mut q = app.world_mut().query::<&MirrorCamera>();
    assert_eq!(
        q.iter(app.world()).count(),
        0,
        "no player vehicle, no mirror cameras"
    );
    let mut ambient = app
        .world_mut()
        .query::<&opts_bevy::mirrors::AmbientCamera>();
    let served = ambient
        .iter(app.world())
        .filter(|c| c.pane.is_some())
        .count();
    eprintln!("ambient cameras with a pane: {served}");
    assert!(served >= 1, "another vehicle's mirror is live");

    spawn_test_vehicle(&mut app, &parts, true);
    app.update();
    app.update();
    let mut q = app.world_mut().query::<&MirrorCamera>();
    let groups: Vec<_> = q.iter(app.world()).map(|c| c.members.len()).collect();
    eprintln!("mirror groups (panes each): {groups:?}");
    assert!(
        groups.len() >= 3,
        "reflexion0, 1 and 3 at least: {groups:?}"
    );
}

