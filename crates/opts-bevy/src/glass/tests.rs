//! Unit tests of `glass.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

fn blended() -> StandardMaterial {
    StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.8,
        reflectance: 0.5,
        base_color: Color::srgba(0.8, 0.9, 1.0, 0.4),
        ..default()
    }
}

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

fn test_app() -> App {
    let mut app = App::new();
    app.init_resource::<Assets<StandardMaterial>>()
        .insert_resource(LightQuality::default())
        .init_resource::<GlassStats>()
        .add_systems(Update, sync_glass);
    app
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

fn window_image(rgba: [u8; 4]) -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let data: Vec<u8> = (0..16).flat_map(|_| rgba).collect();
    Image::new(
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

/// The milk-glass bug of the Hamburg buses: a pale, mostly transparent window texture added
/// at full weight by `Premultiplied` turned the view through the glass white.
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
