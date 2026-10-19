# 🎓 Loading Your First Scenery Object (`.sco`) in Bevy

| Metadata | Value |
| :--- | :--- |
| **Topic** | Scenery Loading & Bevy ECS Spawning |
| **Prerequisites** | Rust 1.76+, basic Bevy ECS concepts, [Coordinate Systems](../architecture/coordinates.md) |
| **Crates Used** | `opts-core`, `opts-bevy` |
| **Example Code** | [`crates/opts-bevy/examples/load_sco.rs`](../../../crates/opts-bevy/examples/load_sco.rs) |

---

## 🎯 What You Will Learn

In this tutorial, you will:
1. Understand the structure of an OMSI 2 scenery object (`.sco`).
2. Parse `.sco` asset data using [`opts_core::formats::scenery::SceneryObject`](../../../crates/opts-omsi/src/formats/scenery/sco.rs).
3. Handle coordinate transformations between OMSI (right-handed $Z$-up) and Bevy (right-handed $Y$-up).
4. Spawn a complete scenery object entity hierarchy in Bevy with mesh attachments and materials using [`spawn_scenery_object`](../../../crates/opts-bevy/src/scenery.rs).

---

## 📖 1. What is a `.sco` Scenery Object?

In OMSI 2, maps are populated by **scenery objects** (`.sco` files). A scenery object represents a world item such as:
- A building, house, or bus shelter.
- A traffic light head, streetlight, or traffic sign.
- A road junction with AI navigation paths.

A `.sco` file defines:
- **Identity & Editor Metadata:** Friendly display name (`[friendlyname]`), editor categories (`[groups]`), complexity priority (`[complexity]`).
- **3D Mesh Attachments:** References to 3D meshes (`.o3d` or `.x`) via repeating `[mesh]` directives.
- **Material Modifiers:** Overriding textures, alpha transparency modes (`[matl_alpha]`), lightmaps, and night textures (`[matl_nightmap]`).
- **Collision & Bounding Boxes:** Physical extents (`[boundingbox]`) and collision geometry (`[collision_mesh]`).
- **Traffic Paths:** AI vehicle and pedestrian lanes (`[path]`).

---

## 🔄 2. Coordinate System Conversion

OMSI uses a **right-handed $Z$-up** world frame:
- $+X$: Right (lateral easting)
- $+Y$: Forward (longitudinal northing)
- $+Z$: Up (vertical elevation)

Bevy uses a **right-handed $Y$-up** world frame:
- $+X$: Right
- $+Y$: Up
- $-Z$: Forward
- $+Z$: Backwards (toward the camera)

`opts-bevy` provides the [`OmsiTransform`](../../../crates/opts-bevy/src/coords.rs) component and [`OptsCoordsPlugin`](../../../crates/opts-bevy/src/plugin.rs), which automatically synchronize OMSI coordinates with Bevy's native `Transform` component:

```rust
use opts_bevy::{omsi_to_bevy_pos, bevy_to_omsi_pos};
use bevy::math::Vec3;

// An OMSI bus stop shelter at X=10m right, Y=50m forward, Z=2m elevation:
let omsi_pos = Vec3::new(10.0, 50.0, 2.0);
let bevy_pos = omsi_to_bevy_pos(omsi_pos);

// In Bevy coordinates: X=10m, Y=2m (up), Z=-50m (forward):
assert_eq!(bevy_pos, Vec3::new(10.0, 2.0, -50.0));
```

---

## 🛠️ 3. Step-by-Step Implementation

### Step 1: Add Dependencies

Add `opts-core` and `opts-bevy` to your `Cargo.toml`:

```toml
[dependencies]
bevy = "0.19"
opts-core = { path = "crates/opts-core" }
opts-bevy = { path = "crates/opts-bevy" }
```

### Step 2: Initialize Bevy with `OptsAssetPlugin`

The [`OptsAssetPlugin`](../../../crates/opts-bevy/src/plugin.rs) bundles all asset loaders (`.o3d`, `.x`, `.dds`, `.bmp`, `.tga`, `.sco`, `.sli`) and registers the coordinate synchronization systems:

```rust
use bevy::prelude::*;
use opts_bevy::OptsAssetPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(OptsAssetPlugin)
        .add_systems(Startup, setup_scene)
        .run();
}
```

### Step 3: Spawn the Scenery Object

In your startup system, parse the `.sco` file and call [`spawn_scenery_object`](../../../crates/opts-bevy/src/scenery.rs):

```rust
use bevy::prelude::*;
use opts_bevy::spawn_scenery_object;
use opts_core::formats::scenery::SceneryObject;

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 1. Camera & Lighting
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 3.0, 8.0)
            .looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.6, 0.4, 0.0)),
        ..default()
    });

    // 2. Parse the scenery object file
    let sco_raw_bytes = include_bytes!("../../assets/sample_shelter.sco");
    let sco = SceneryObject::parse(sco_raw_bytes).expect("Failed to parse .sco file");

    // 3. Prepare mesh and material handles for each [mesh] attachment
    let base_mesh: Handle<Mesh> = meshes.add(Cuboid::new(2.5, 2.2, 1.2));
    let base_material: Handle<StandardMaterial> = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.25, 0.3),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    let mesh_handles = vec![base_mesh];
    let material_handles = vec![base_material];

    // 4. Spawn the entity hierarchy
    let spawn_transform = Transform::from_xyz(0.0, 1.1, 0.0);
    let entity = spawn_scenery_object(
        &mut commands,
        &sco,
        spawn_transform,
        &mesh_handles,
        &material_handles,
    );

    println!("Spawned Scenery Object entity: {:?}", entity);
}
```

---

## 🏃 4. Running the Example

An interactive, runnable version of this tutorial is included directly in `opts-bevy`. Run it with:

```bash
cargo run -p opts-bevy --example load_sco
```

You should see a window displaying the spawned scenery object hierarchy with lighting and camera controls.

---

## 🔗 Next Steps

- [Loading a Full Bus Model (`first-bus.md`)](first-bus.md) *(coming soon)*
- [OMSI Scenery Objects (`.sco`) Specification](../formats/text/sco.md)
- [Spline Profile Extrusions (`.sli`) Specification](../formats/text/sli.md)
- [Coordinate Systems & Spatial Math](../architecture/coordinates.md)
