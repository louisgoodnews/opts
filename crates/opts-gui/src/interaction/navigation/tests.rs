//! Unit tests for `super` (`navigation.rs`).

use super::*;

fn cand(i: u32, region: Option<u32>, rect: Option<Rect>) -> Candidate {
    Candidate {
        entity: Entity::from_raw_u32(i).expect("entity"),
        order: vec![i],
        rect,
        region: region.map(|r| Entity::from_raw_u32(1000 + r).expect("entity")),
    }
}

fn r(x: f32, y: f32) -> Option<Rect> {
    Some(Rect::from_center_size(
        Vec2::new(x, y),
        Vec2::new(80.0, 32.0),
    ))
}

fn e(i: u32) -> Entity {
    Entity::from_raw_u32(i).expect("entity")
}

#[test]
fn linear_wraps_in_both_directions() {
    let c = [
        cand(1, None, None),
        cand(2, None, None),
        cand(3, None, None),
    ];
    assert_eq!(next_linear(&c, Some(e(3)), false), Some(e(1)));
    assert_eq!(next_linear(&c, Some(e(1)), true), Some(e(3)));
    assert_eq!(next_linear(&c, None, false), Some(e(1)));
    assert_eq!(next_linear(&c, None, true), Some(e(3)));
    assert_eq!(next_linear(&[], None, false), None);
}

#[test]
fn regions_jump_and_fall_back_to_linear() {
    let c = [
        cand(1, Some(0), None),
        cand(2, Some(0), None),
        cand(3, Some(1), None),
    ];
    assert_eq!(next_region(&c, Some(e(1)), false), Some(e(3)));
    assert_eq!(next_region(&c, Some(e(3)), false), Some(e(1)));
    assert_eq!(next_region(&c, Some(e(1)), true), Some(e(3)));
    let one = [cand(1, None, None), cand(2, None, None)];
    assert_eq!(next_region(&one, Some(e(1)), false), Some(e(2)));
}

#[test]
fn directional_keeps_rows_and_columns_straight() {
    // a grid: 1 2 / 3 4
    let c = [
        cand(1, None, r(100.0, 100.0)),
        cand(2, None, r(300.0, 100.0)),
        cand(3, None, r(100.0, 200.0)),
        cand(4, None, r(300.0, 200.0)),
    ];
    assert_eq!(move_focus(&c, Some(e(1)), Direction::Right), Some(e(2)));
    assert_eq!(move_focus(&c, Some(e(1)), Direction::Down), Some(e(3)));
    assert_eq!(move_focus(&c, Some(e(4)), Direction::Left), Some(e(3)));
    assert_eq!(move_focus(&c, Some(e(4)), Direction::Up), Some(e(2)));
    assert_eq!(
        move_focus(&c, Some(e(2)), Direction::Right),
        None,
        "edge: stay"
    );
}

#[test]
fn directional_stays_inside_the_region_while_it_has_a_target() {
    let c = [
        cand(1, Some(0), r(100.0, 100.0)),
        cand(2, Some(1), r(200.0, 100.0)),
        cand(3, Some(0), r(350.0, 100.0)),
    ];
    // 2 is nearer but belongs to another region (cost 200): 3 wins.
    assert_eq!(move_focus(&c, Some(e(1)), Direction::Right), Some(e(3)));
}

#[test]
fn directional_crosses_into_the_next_region_at_the_edge() {
    let c = [
        cand(1, Some(0), r(100.0, 100.0)),
        cand(2, Some(1), r(300.0, 100.0)),
    ];
    assert_eq!(move_focus(&c, Some(e(1)), Direction::Right), Some(e(2)));
    assert_eq!(move_focus(&c, Some(e(2)), Direction::Left), Some(e(1)));
    assert_eq!(
        move_focus(&c, Some(e(1)), Direction::Up),
        None,
        "nothing above"
    );
}

#[test]
fn without_geometry_directions_follow_layout_order() {
    let c = [cand(1, None, None), cand(2, None, None)];
    assert_eq!(move_focus(&c, Some(e(1)), Direction::Down), Some(e(2)));
    assert_eq!(
        move_focus(&c, Some(e(1)), Direction::Left),
        Some(e(2)),
        "wraps"
    );
}
