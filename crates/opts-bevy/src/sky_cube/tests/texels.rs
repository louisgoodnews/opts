//! The sky's radiance, the cube's data layout and the face directions.

use super::*;

#[test]
fn the_sky_gets_brighter_towards_its_colours_and_the_sun_spot_is_there() {
    let i = input();
    let up = sky_radiance(Vec3::Y, &i);
    let hor = sky_radiance(Vec3::X, &i);
    assert!((up - Vec3::from(i.zenith)).length() < 0.05, "zenith {up:?}");
    assert!(
        (hor - Vec3::from(i.horizon)).length() < 0.05,
        "horizon {hor:?}"
    );
    let spot = sky_radiance(i.sun_dir, &i);
    assert!(spot.max_element() > 10.0, "sun spot {spot:?}");
    let down = sky_radiance(-Vec3::Y, &i);
    assert!(down.max_element() < hor.min_element(), "ground is darker");
}

#[test]
fn no_sun_spot_below_the_horizon() {
    let mut i = input();
    i.sun_elevation_deg = -20.0;
    i.sun_dir = Vec3::new(0.3, -0.6, 0.4).normalize();
    assert!(sky_radiance(i.sun_dir, &i).max_element() < 1.0);
}

#[test]
fn the_cube_data_has_six_faces_of_rgba16f() {
    let d = cube_data(&input(), 4);
    assert_eq!(d.len(), 4 * 4 * 6 * 8);
}

#[test]
fn face_directions_are_unit_and_cover_all_axes() {
    for face in 0..6 {
        let d = face_direction(face, 1, 2, 8);
        assert!((d.length() - 1.0).abs() < 1e-5);
    }
    assert!(face_direction(0, 4, 4, 8).x > 0.9);
    assert!(face_direction(1, 4, 4, 8).x < -0.9);
    assert!(face_direction(2, 4, 4, 8).y > 0.9);
    assert!(face_direction(3, 4, 4, 8).y < -0.9);
    assert!(face_direction(4, 4, 4, 8).z > 0.9);
    assert!(face_direction(5, 4, 4, 8).z < -0.9);
}
