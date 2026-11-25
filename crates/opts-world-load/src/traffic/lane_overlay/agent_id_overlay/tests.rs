//! Unit tests of the agent-id overlay's pure functions.

use super::*;

#[test]
fn a_label_shows_the_agent_id() {
    assert_eq!(label_text(42), "#42");
}

#[test]
fn the_nearest_vehicles_within_the_radius_come_first_and_are_capped() {
    let agents = [
        (1, DVec2::new(100.0, 0.0), 0.0),
        (2, DVec2::new(10.0, 0.0), 0.0),
        (3, DVec2::new(500.0, 0.0), 0.0),
        (4, DVec2::new(30.0, 0.0), 0.0),
    ];
    let got: Vec<u64> = nearest_agents(&agents, DVec2::ZERO, 150.0, 2)
        .iter()
        .map(|a| a.0)
        .collect();
    assert_eq!(got, vec![2, 4]);
}
