//! Tests of the door camera pose and of the door grouping by unit and zone.

use super::*;
#[test]
fn the_camera_hangs_at_the_upper_rear_of_a_right_door_and_the_upper_front_of_a_left_one() {
    let right = DoorPoint {
        entry: true,
        index: 0,
        position: Vec3::new(1.2, 0.0, -3.0),
        side: 1,
        unit: None,
    };
    let left = DoorPoint {
        side: -1,
        position: Vec3::new(-1.2, 0.0, -3.0),
        ..right.clone()
    };
    let r = door_pose(&right);
    let l = door_pose(&left);
    // Bevy: z backward. The right door's camera is behind it, the left one's in front of it.
    assert!(r.translation.z > right.position.z && l.translation.z < left.position.z);
    assert!(r.translation.x > right.position.x && l.translation.x < left.position.x);
    assert!(r.translation.y > 1.5, "above the door");
    // Both look at the doorway: the door point lies in front of the lens.
    for (pose, door) in [(r, &right), (l, &left)] {
        let to_door = (door.position + Vec3::Y * 0.7 - pose.translation).normalize();
        assert!(pose.forward().dot(to_door) > 0.99);
    }
}

#[test]
fn the_points_of_one_entrance_are_one_door() {
    let p = |z: f32, side: i8| DoorPoint {
        entry: true,
        index: 0,
        position: Vec3::new(f32::from(side), 0.0, z),
        side,
        unit: None,
    };
    // Two lanes of the front door, two of the rear door, one on the left.
    let d = vec![p(-5.3, 1), p(-4.6, 1), p(-0.7, 1), p(-0.2, 1), p(-5.0, -1)];
    let groups: Vec<usize> = (0..d.len()).map(|i| door_group(&d, i)).collect();
    assert_eq!(groups, vec![0, 0, 2, 2, 4]);
}

#[test]
fn doors_of_another_unit_are_not_grouped_with_the_heads_and_zone_within_their_unit() {
    let unit = Entity::from_raw_u32(7).expect("entity");
    let mut d = doors();
    for p in doors() {
        d.push(DoorPoint {
            unit: Some(unit),
            ..p
        });
    }
    assert_eq!(door_group(&d, 5), 4);
    assert_eq!(DoorPoints(d).zone(4), DoorPoints(doors()).zone(0));
}
