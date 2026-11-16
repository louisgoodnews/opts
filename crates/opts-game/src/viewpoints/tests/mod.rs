//! Unit tests of `viewpoints`.

use super::*;

fn vp(x: f32, fov: Option<f32>) -> Viewpoint {
    Viewpoint {
        x,
        y: 0.05,
        z: -0.1,
        fov_deg: fov,
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/viewpoint-tests")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

mod editor;
mod files;
mod models;
mod respawn;
mod store;

fn app(path: PathBuf) -> App {
    let mut app = App::new();
    app.init_resource::<CameraState>()
        .init_resource::<ViewpointState>()
        .insert_resource(Viewpoints {
            path: Some(path),
            store: ViewpointStore::default(),
        })
        .add_message::<RaiseAlert>()
        .add_systems(Update, (load_viewpoint, persist_viewpoint).chain());
    app
}

fn spawn_vehicle(app: &mut App, key: &str) -> Entity {
    app.world_mut()
        .spawn((PlayerVehicle, ViewpointKey(key.to_owned())))
        .id()
}

fn edit_and_save(app: &mut App, change: impl FnOnce(&mut ViewpointState)) {
    let mut st = app.world_mut().resource_mut::<ViewpointState>();
    st.open();
    change(&mut st);
    st.save();
}
