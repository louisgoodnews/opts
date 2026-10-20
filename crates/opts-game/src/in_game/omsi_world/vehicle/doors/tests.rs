//! Tests of the door points of a cabin.

use super::*;

const CABIN: &str = "[entry]\n3\n\n[entry]\n0\n\n[exit]\n23\n\n[exit]\n21\n\n[exit]\n99\n";

fn paths(points: &[(f32, f32)]) -> PassengerPaths {
    let mut text = String::new();
    for (x, y) in points {
        text.push_str(&format!("[pathpnt]\n{x}\n{y}\n0.4\n\n"));
    }
    PassengerPaths::parse(&text).expect("paths")
}

fn points() -> Vec<DoorPoint> {
    let mut pts = vec![(0.0, 0.0); 24];
    pts[0] = (1.25, 4.2);
    pts[3] = (1.25, 4.9);
    pts[21] = (-1.2, -2.5);
    pts[23] = (1.25, -2.4);
    door_points(CABIN, &paths(&pts))
}

#[test]
fn the_doors_are_the_entries_then_the_exits_that_have_a_pathpoint() {
    let d = points();
    let order: Vec<(bool, usize)> = d.iter().map(|p| (p.entry, p.index)).collect();
    assert_eq!(
        order,
        [(true, 3), (true, 0), (false, 23), (false, 21)],
        "99 has no point"
    );
    // Vehicle space (x right, y forward, z up) became Bevy axes (x right, y up, -z forward).
    assert_eq!(d[0].position, Vec3::new(1.25, 0.4, -4.9));
    assert_eq!((d[0].side, d[3].side), (1, -1));
}

#[test]
fn a_bad_file_gives_no_doors() {
    assert!(door_points_from_bytes(b"[entry]\n0\n", b"[pathpnt]\nx\ny\nz\n").is_empty());
    assert!(door_points_from_bytes(b"", b"").is_empty());
}
