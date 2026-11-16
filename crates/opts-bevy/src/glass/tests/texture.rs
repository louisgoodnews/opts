//! Glass texture tests: a pale window texture faded by its alpha, and the fallback when it cannot be read.

use super::*;

#[test]
fn a_pale_window_texture_is_faded_by_its_alpha_while_the_look_is_on() {
    let mut app = test_app();
    app.init_resource::<Assets<Image>>();
    let texture = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(window_image([230, 235, 240, 40]));
    let material = StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        base_color_texture: Some(texture.clone()),
        ..default()
    };
    let h = unique_material(
        &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
        material.clone(),
    );
    note_surface(&h, Some("fenster.tga"), &material, false);
    app.update();
    let shown = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&h)
        .unwrap()
        .base_color_texture
        .clone()
        .expect("texture");
    assert_ne!(shown.id(), texture.id(), "the premultiplied copy is shown");
    let images = app.world().resource::<Assets<Image>>();
    let copy = images.get(&shown).unwrap();
    let px = &copy.data.as_ref().unwrap()[..4];
    assert!(
        px[0] < 120,
        "diffuse added over the view must be faint: {px:?}"
    );
    assert_eq!(px[3], 40);
    assert_eq!(images.get(&texture).unwrap().data.as_ref().unwrap()[0], 230);

    app.world_mut()
        .resource_mut::<LightQuality>()
        .glass_reflections = false;
    app.update();
    let back = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&h)
        .unwrap();
    assert_eq!(
        back.base_color_texture.as_ref().map(|t| t.id()),
        Some(texture.id())
    );
    assert_eq!(back.alpha_mode, AlphaMode::Blend);
}

#[test]
fn glass_whose_texture_cannot_be_read_gets_a_small_diffuse_instead() {
    let mut app = test_app();
    app.init_resource::<Assets<Image>>();
    let missing = Handle::<Image>::Uuid(bevy::asset::uuid::Uuid::from_u128(99), default());
    let material = StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        base_color_texture: Some(missing.clone()),
        ..default()
    };
    let h = unique_material(
        &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
        material.clone(),
    );
    note_surface(&h, Some("glas.tga"), &material, false);
    app.update();
    let m = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&h)
        .unwrap();
    assert_eq!(m.alpha_mode, AlphaMode::Premultiplied);
    assert_eq!(
        m.base_color_texture.as_ref().map(|t| t.id()),
        Some(missing.id())
    );
    assert!((m.base_color.to_linear().red - GLASS_FALLBACK_TINT * GLASS_CLARITY).abs() < 1e-6);
}
