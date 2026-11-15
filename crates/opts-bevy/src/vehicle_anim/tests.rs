use super::*;

pub(super) fn spin(var: &str) -> AnimStep {
    AnimStep {
        frame: Mat4::from_translation(Vec3::new(1.2, 2.9, 0.5)),
        variable: var.into(),
        factor: 57.295_78,
        translate: false,
        maxspeed: None,
        delay: None,
    }
}

/// A point of the mesh in Bevy axes from OMSI coordinates.
fn bevy(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

mod dash;
mod motion;
mod steering;
mod wheel;

fn steering_mesh(cfg: &str, pivot: Option<MeshPivot>) -> MeshAnimation {
    let model = opts_core::formats::vehicles::model::ModelConfig::parse(cfg).expect("parses");
    MeshAnimation::from_mesh(Entity::PLACEHOLDER, &model.lods[0].meshes[0], pivot)
        .expect("animated")
}

/// Whether a positive steering variable turns a steering wheel with the rim normal `normal`
/// clockwise as the driver sees it (looking down the column): a point on the rim moves to the
/// right of where it was.
fn turns_clockwise(step: &AnimStep, normal: Vec3) -> bool {
    let n = if normal.z < 0.0 { -normal } else { normal };
    let u = (Vec3::Y - n * Vec3::Y.dot(n)).normalize();
    let m = animation_matrix_from_values(std::slice::from_ref(step), &[0.02]);
    let c = bevy(Vec3::ZERO);
    let moved = m.transform_point3(bevy(u)) - m.transform_point3(c);
    // Back to OMSI axes.
    let moved = Vec3::new(moved.x, -moved.z, moved.y);
    u.cross(moved).dot(n) < 0.0
}
