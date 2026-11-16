//! Tests of the free-texture binding: texture names, caching and the lit copy.

use super::*;
use opts_sim::script::ScriptEngine;

#[test]
fn the_part_shows_the_texture_its_variable_names_on_its_own_material() {
    let dir = std::env::temp_dir().join(format!("opts_free_texture_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Step3")).unwrap();
    image::RgbaImage::from_pixel(4, 4, image::Rgba([10, 200, 30, 255]))
        .save(dir.join("Step3/Page.png"))
        .unwrap();

    let mut app = App::new();
    app.init_resource::<Assets<Image>>()
        .init_resource::<Assets<StandardMaterial>>()
        .add_systems(Update, free_texture_system);
    let (engine, _) = ScriptEngine::from_sources(&[("t.osc", "{init}\n{end}\n")]);
    let scripts = VehicleScripts::new(engine);
    scripts
        .engine
        .lock()
        .unwrap()
        .set_string("page", "step3\\PAGE.png");
    let owner = app.world_mut().spawn(scripts).id();
    let shared = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    let part = app
        .world_mut()
        .spawn((
            FreeTextureBinding::new(owner, "page".into(), vec![dir.clone()], None),
            MeshMaterial3d(shared.clone()),
        ))
        .id();
    app.update();

    let material = app
        .world()
        .get::<MeshMaterial3d<StandardMaterial>>(part)
        .unwrap()
        .0
        .clone();
    assert_ne!(material, shared, "the part got its own material");
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let texture = materials
        .get(&material)
        .unwrap()
        .base_color_texture
        .clone()
        .unwrap();
    assert!(
        materials.get(&shared).unwrap().base_color_texture.is_none(),
        "the shared one is untouched"
    );
    let img = app
        .world()
        .resource::<Assets<Image>>()
        .get(&texture)
        .unwrap();
    assert_eq!(&img.data.as_deref().unwrap()[0..3], &[10, 200, 30]);
    let _ = std::fs::remove_dir_all(&dir);
}
