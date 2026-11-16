//! Glass setting tests: the classification of names, the setting and the restore of the look.

use super::*;

#[test]
fn names_are_classified() {
    assert_eq!(classify("Fenster.tga", true), Some(SurfaceKind::Glass));
    assert_eq!(classify("Fenster.tga", false), None, "opaque is not glass");
    assert_eq!(
        classify("Texture\\Reflexion0.bmp", false),
        Some(SurfaceKind::Mirror)
    );
    assert_eq!(classify("reflexion3.bmp", true), Some(SurfaceKind::Mirror));
    assert_eq!(classify("reflexion12.bmp", true), Some(SurfaceKind::Mirror));
    // The housing of a mirror unit is not the pane.
    assert_eq!(classify("nlc_side_mirror.dds", true), None);
    assert_eq!(classify("sl_mirrors.dds", false), None);
    assert_eq!(classify("o3d_spiegel.dds", true), None);
    assert_eq!(classify("reflexion.bmp", false), None, "needs its number");
    assert_eq!(classify("reflexion_x.bmp", false), None);
    assert_eq!(classify("verkehrsspiegel.bmp", false), None);
    assert_eq!(classify("karosserie.bmp", true), None);
}
#[test]
fn glass_follows_the_setting_and_restores_exactly() {
    let mut app = test_app();
    let h = unique_material(
        &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
        blended(),
    );
    let original = blended();
    note_surface(&h, Some("fenster_int.tga"), &original, false);
    assert_eq!(surface_kind(h.id()), Some(SurfaceKind::Glass));

    app.update();
    {
        let mats = app.world().resource::<Assets<StandardMaterial>>();
        let m = mats.get(&h).unwrap();
        assert_eq!(m.alpha_mode, AlphaMode::Premultiplied);
        assert!(m.perceptual_roughness <= GLASS_ROUGHNESS + 1e-6);
        assert!(m.reflectance >= GLASS_REFLECTANCE - 1e-6);
        assert!(m.base_color.to_linear().red < original.base_color.to_linear().red);
        assert!(
            (m.base_color.alpha() - 0.4 * GLASS_CLARITY).abs() < 1e-6,
            "alpha scaled by the clarity"
        );
    }

    app.world_mut()
        .resource_mut::<LightQuality>()
        .glass_reflections = false;
    app.update();
    let mats = app.world().resource::<Assets<StandardMaterial>>();
    let m = mats.get(&h).unwrap();
    assert_eq!(m.alpha_mode, AlphaMode::Blend);
    assert_eq!(m.perceptual_roughness, 0.8);
    assert_eq!(m.reflectance, 0.5);
    assert_eq!(m.base_color, original.base_color);
}

#[test]
fn with_reflections_off_a_new_glass_material_stays_untouched() {
    let mut app = test_app();
    app.world_mut()
        .resource_mut::<LightQuality>()
        .glass_reflections = false;
    let h = unique_material(
        &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
        blended(),
    );
    note_surface(&h, Some("glass.tga"), &blended(), false);
    app.update();
    let m = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&h)
        .unwrap();
    assert_eq!(m.alpha_mode, AlphaMode::Blend);
    assert_eq!(m.perceptual_roughness, 0.8);
}
