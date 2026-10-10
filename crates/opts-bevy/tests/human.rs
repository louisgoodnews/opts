//! Human rig: eye anchor height, mirrored pivots and posed limbs in world space.

use bevy::asset::AssetPlugin;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use opts_bevy::{spawn_human, Human, HumanHead, HumanPose, HUMAN_BONES};
use opts_core::formats::humans::HumanProfile;

const HUM: &str = "[model]\nm.cfg\n[seatheight]\n0.81\n[humangeom]\n0.04\n1.77\n[links]\n0.09\n0.00\n0.92\n0.09\n-0.03\n0.53\n0.02\n1.17\n0.18\n-0.05\n1.43\n0.44\n-0.04\n1.41\n-0.02\n1.55\n0.69\n-0.03\n1.43\n0.90\n-0.03\n1.43\n[walk_param]\n1.4\n70\n0.7\n1.0\n0.0\n[voice]\nM1\n[mass]\n0.07\n";

fn world_pos(app: &App, e: Entity) -> Vec3 {
    app.world().get::<GlobalTransform>(e).unwrap().translation()
}

#[derive(Resource)]
struct RootEntity(Entity);

fn build(pose: HumanPose) -> (App, Human) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            watch_for_changes_override: Some(false),
            ..Default::default()
        },
        bevy::transform::TransformPlugin,
    ))
    .init_asset::<SkinnedMeshInverseBindposes>()
    .add_systems(
        Startup,
        move |mut cmds: Commands, mut ibp: ResMut<Assets<SkinnedMeshInverseBindposes>>| {
            let profile = HumanProfile::from_text(HUM).unwrap();
            let root = spawn_human(&mut cmds, &mut ibp, &profile, &[], "test", pose);
            cmds.insert_resource(RootEntity(root));
        },
    );
    app.update();
    app.update();
    let root = app.world().resource::<RootEntity>().0;
    let human = app.world().get::<Human>(root).unwrap().clone();
    (app, human)
}

#[test]
fn eye_anchor_sits_at_stature_minus_eye_offset_in_both_poses() {
    for pose in [HumanPose::Standing, HumanPose::Seated] {
        let (app, human) = build(pose);
        let eye = world_pos(&app, human.rig.head_anchor);
        assert!(
            (eye.y - (1.77 - 0.11)).abs() < 1e-4,
            "{pose:?}: eye height {}",
            eye.y
        );
        assert!(app
            .world()
            .get::<HumanHead>(human.rig.head_anchor)
            .is_some());
    }
}

#[test]
fn standing_arms_hang_and_seated_knees_move_forward() {
    let (standing, hs) = build(HumanPose::Standing);
    let joint =
        |h: &Human, name: &str| h.rig.joints[HUMAN_BONES.iter().position(|b| *b == name).unwrap()];
    let shoulder = world_pos(&standing, joint(&hs, "OA_R"));
    let hand = world_pos(&standing, joint(&hs, "Hand_R"));
    assert!(
        hand.y < shoulder.y - 0.3,
        "hand below shoulder: {hand:?} vs {shoulder:?}"
    );

    let (seated, hq) = build(HumanPose::Seated);
    let hip = world_pos(&seated, joint(&hq, "OS_R"));
    let knee = world_pos(&seated, joint(&hq, "US_R"));
    let (stand_hip, stand_knee) = (
        world_pos(&standing, joint(&hs, "OS_R")),
        world_pos(&standing, joint(&hs, "US_R")),
    );
    assert!((knee.y - hip.y).abs() < 0.05, "seated thigh is horizontal");
    assert!(
        knee.z < hip.z - 0.3,
        "knee is in front of the hip (Bevy forward is -Z)"
    );
    assert!(
        stand_knee.y < stand_hip.y - 0.3,
        "standing thigh is vertical"
    );
}
