//! Unit tests for `super` (`tree_wind.rs`).

use super::*;

#[test]
fn the_wind_follows_the_weather_and_the_setting() {
    let calm = wind_uniform(true, Vec2::new(1.0, 0.0), 0.0);
    assert!(
        (calm.z - CALM_M).abs() < 1e-6,
        "a calm day still moves a little"
    );
    assert_eq!(Vec2::new(calm.x, calm.y), CALM_DIRECTION);
    let breeze = wind_uniform(true, Vec2::new(0.0, -2.0), 5.0);
    assert_eq!(Vec2::new(breeze.x, breeze.y), Vec2::new(0.0, -1.0));
    assert!(breeze.z > calm.z);
    let storm = wind_uniform(true, Vec2::X, 18.9);
    assert!((storm.z - 0.5125).abs() < 1e-4, "{}", storm.z);
    assert!(wind_uniform(true, Vec2::X, 100.0).z <= MAX_M);
    assert_eq!(wind_uniform(false, Vec2::X, 18.9).z, 0.0, "off: still");
}

#[test]
fn a_changed_wind_reaches_every_tree_material() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<TreeMaterial>()
        .init_resource::<TreeWindState>()
        .add_systems(Update, update_tree_wind);
    let state = *app.world().resource::<TreeWindState>();
    let a = app
        .world_mut()
        .resource_mut::<Assets<TreeMaterial>>()
        .add(TreeMaterial {
            base: crate::tree_billboard_material(Handle::default()),
            extension: TreeWind {
                wind: state.wind,
                shape: BILLBOARD,
            },
        });
    app.update();
    app.world_mut().resource_mut::<TreeWindState>().enabled = false;
    app.update();
    let m = app
        .world()
        .resource::<Assets<TreeMaterial>>()
        .get(&a)
        .unwrap();
    assert_eq!(m.extension.wind.z, 0.0);
    app.world_mut().resource_mut::<TreeWindState>().enabled = true;
    app.update();
    let m = app
        .world()
        .resource::<Assets<TreeMaterial>>()
        .get(&a)
        .unwrap();
    assert!(m.extension.wind.z > 0.0);
}

#[test]
fn a_marked_tree_gets_the_tree_material_shared_per_texture() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<TreeMaterial>()
        .init_resource::<TreeWindState>()
        .add_systems(Update, adopt_tree_material);
    let base = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(crate::tree_billboard_material(Handle::default()));
    let trees: Vec<Entity> = (0..3)
        .map(|_| {
            app.world_mut()
                .spawn((WindTree, MeshMaterial3d(base.clone())))
                .id()
        })
        .collect();
    let plain = app.world_mut().spawn(MeshMaterial3d(base.clone())).id();
    app.update();
    let handles: Vec<_> = trees
        .iter()
        .map(|e| {
            assert!(app
                .world()
                .get::<MeshMaterial3d<StandardMaterial>>(*e)
                .is_none());
            app.world()
                .get::<MeshMaterial3d<TreeMaterial>>(*e)
                .expect("tree material")
                .0
                .id()
        })
        .collect();
    assert!(
        handles.windows(2).all(|w| w[0] == w[1]),
        "one material per texture"
    );
    assert!(app
        .world()
        .get::<MeshMaterial3d<StandardMaterial>>(plain)
        .is_some());
    assert_eq!(app.world().resource::<Assets<TreeMaterial>>().len(), 1);
}

#[test]
fn a_mesh_trees_foliage_sways_and_its_trunk_stays() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset::<TreeMaterial>()
        .init_resource::<TreeWindState>()
        .add_systems(Update, (tag_mesh_tree_foliage, adopt_tree_material).chain());
    let mut standard = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    let bark = standard.add(StandardMaterial::default());
    let leaves = standard.add(StandardMaterial {
        alpha_mode: AlphaMode::Mask(0.5),
        ..default()
    });
    let world = app.world_mut();
    let trunk = world.spawn(MeshMaterial3d(bark.clone())).id();
    let crown = world.spawn(MeshMaterial3d(leaves.clone())).id();
    let lod = world.spawn_empty().add_child(crown).id();
    world.spawn(MeshTreeRoot).add_children(&[trunk, lod]);
    // The same foliage on an object that is no tree stays still.
    let hedge_card = world.spawn(MeshMaterial3d(leaves)).id();
    world.spawn_empty().add_child(hedge_card);
    app.update();
    let world = app.world();
    assert!(world
        .get::<MeshMaterial3d<StandardMaterial>>(trunk)
        .is_some());
    assert!(world
        .get::<MeshMaterial3d<StandardMaterial>>(hedge_card)
        .is_some());
    let m = world
        .get::<MeshMaterial3d<TreeMaterial>>(crown)
        .expect("the crown sways");
    let m = world.resource::<Assets<TreeMaterial>>().get(&m.0).unwrap();
    assert_eq!(m.extension.shape, MESH_TREE, "height-only bend");
}
