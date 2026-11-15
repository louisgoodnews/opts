//! Attachment runs along a chain: distances, spacing, repeaters across tiles and chain offsets.

use super::super::*;
use super::helpers::*;
use opts_core::formats::map::RepeaterLink;

#[test]
fn single_attachment_distance_is_chain_chainage() {
    let (splines, index) = chain();
    // 70 m along the chain = 20 m into the second spline; 3 m to the right of an east-bound road.
    let mut att = attachment(1, 3.0, 70.0);
    att.string_vars = vec![String::new(), "1".into()];
    let out = expand(&[att], &splines, &index, (0, 0));
    assert_eq!(out.len(), 1);
    assert!((out[0].0.position.x - 70.0).abs() < 1e-3);
    assert!((out[0].0.position.y - 7.0).abs() < 1e-3);
    assert!((out[0].0.position.z - 0.25).abs() < 1e-4);
    assert!((out[0].0.heading_deg - 270.0).abs() < 1e-3);
    assert_eq!(
        out[0].0.string_vars,
        vec!["", "1"],
        "string values are passed on"
    );
}

/// A master places `⌊range / spacing⌋ + 1` instances `spacing` apart along its chain.
#[test]
fn master_run_places_every_spacing_along_the_chain() {
    let (splines, index) = chain();
    let out = expand(&[run(0, 10.0, 20.0, 100.0)], &splines, &index, (0, 0));
    assert_eq!(xs(&out), vec![10.0, 30.0, 50.0, 70.0, 90.0, 110.0]);
    // range 140 / spacing 7 = 20 steps: 21 instances when the chain is long enough.
    let out = expand(&[run(0, 0.0, 7.0, 140.0)], &splines, &index, (0, 0));
    assert_eq!(out.len(), 21);
    // …but never past the chain's end (150 m): 0, 7, …, 147.
    let out = expand(&[run(0, 0.0, 7.0, 400.0)], &splines, &index, (0, 0));
    assert_eq!(out.len(), 22);
    assert!(xs(&out).iter().all(|x| (0.0..=150.0).contains(x)));
}

/// The master stops at its tile's border; the repeater on the next tile continues from its
/// first instance, and a duplicate repeater adds nothing.
#[test]
fn repeater_continues_the_run_on_the_next_tile() {
    let a = vec![spline(1, 0, 2, 0.0, 50.0)];
    let b = vec![spline(2, 1, 0, 50.0, 100.0)];
    let index = index_of(&[((0, 0), a.clone()), ((1, 0), b.clone())]);
    let master = run(0, 10.0, 20.0, 100.0);
    let on_a = expand(std::slice::from_ref(&master), &a, &index, (0, 0));
    assert_eq!(xs(&on_a), vec![10.0, 30.0, 50.0]);
    let rep = MapSplineAttachment {
        repeater: Some(RepeaterLink {
            master_tile: 0,
            first_instance: 3,
        }),
        ..master
    };
    let on_b = expand(&[rep.clone(), rep], &b, &index, (1, 0));
    // Instances 3..=5 at chainages 70, 90, 110 = 20, 40, 60 m into spline 2 (tile frame x).
    assert_eq!(xs(&on_b), vec![70.0, 90.0, 110.0]);
}

#[test]
fn chain_offsets_match_a_plain_walk_also_in_loops() {
    let splines = vec![
        spline(1, 0, 2, 0.0, 50.0),
        spline(2, 1, 3, 50.0, 100.0),
        spline(3, 2, 0, 150.0, 25.0),
        // A ring of three.
        spline(10, 12, 11, 0.0, 10.0),
        spline(11, 10, 12, 0.0, 20.0),
        spline(12, 11, 10, 0.0, 30.0),
    ];
    let index = index_of(&[((0, 0), splines.clone())]);
    for s in &splines {
        assert_eq!(
            index.chain_offset(s.id_code),
            index.walk_offset(s.id_code),
            "{}",
            s.id_code
        );
    }
    assert_eq!(index.chain_offset(3), 150.0);
    assert_eq!(index.chain_offset(10), 50.0);
}

#[test]
fn attachment_past_chain_end_is_dropped() {
    let (splines, index) = chain();
    let out = expand(&[attachment(1, 0.0, 400.0)], &splines, &index, (0, 0));
    assert!(out.is_empty());
}
