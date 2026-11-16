//! Glass pixel tests: transmap glass and the premultiplied colour of a pale window.

use super::*;

#[test]
fn the_glass_pass_of_a_transmap_split_is_glass_by_force() {
    let mut app = test_app();
    let h = unique_material(
        &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
        blended(),
    );
    note_surface(&h, Some("bus_body.bmp"), &blended(), true);
    assert_eq!(surface_kind(h.id()), Some(SurfaceKind::Glass));
    let opaque = StandardMaterial::default();
    let h2 = unique_material(
        &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
        opaque.clone(),
    );
    note_surface(&h2, Some("fenster.tga"), &opaque, false);
    assert_eq!(
        surface_kind(h2.id()),
        None,
        "an opaque window is not touched"
    );
}

#[test]
fn premultiplying_fades_the_colour_by_alpha_in_linear_light() {
    // A white texel at alpha 128 (about 0.5) is sRGB 188 once multiplied in linear light, not 128.
    let mut px = [255, 255, 255, 128, 10, 20, 30, 255, 200, 100, 50, 0];
    premultiply_rgba8(&mut px, true);
    assert!((i32::from(px[0]) - 188).abs() <= 1, "{}", px[0]);
    assert_eq!(px[3], 128, "alpha is not touched");
    assert_eq!(&px[4..8], &[10, 20, 30, 255], "opaque texels stay");
    assert_eq!(&px[8..11], &[0, 0, 0], "transparent texels turn black");
    let mut linear = [200, 100, 50, 128];
    premultiply_rgba8(&mut linear, false);
    assert_eq!(&linear[..3], &[100, 50, 25]);
}
