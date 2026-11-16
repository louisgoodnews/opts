//! The LOD bands follow the screen size of the object.

use super::*;

#[test]
fn lod_shows_the_band_of_the_current_screen_size() {
    let mut app = app();
    let groups = {
        let root = app.world().resource::<Root>().0;
        app.world().get::<SceneryLod>(root).unwrap().groups.clone()
    };
    let (fine, coarse) = (groups[0], groups[1]);
    assert_eq!((fine.1, coarse.1), (0.2, 0.0));
    app.update();
    assert_eq!(visibility(&app, fine.0), Visibility::Inherited);
    assert_eq!(visibility(&app, coarse.0), Visibility::Hidden);

    // Far away the object is small: the stand-in shows instead.
    let mut q = app
        .world_mut()
        .query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
    for mut t in q.iter_mut(app.world_mut()) {
        *t = Transform::from_xyz(0.0, 1.0, 400.0).looking_at(Vec3::ZERO, Vec3::Y);
    }
    app.update();
    app.update();
    assert_eq!(visibility(&app, fine.0), Visibility::Hidden);
    assert_eq!(visibility(&app, coarse.0), Visibility::Inherited);
}
