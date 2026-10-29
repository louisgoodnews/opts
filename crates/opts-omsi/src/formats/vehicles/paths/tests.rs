//! Tests for passenger paths parsing.

use super::*;

#[test]
fn test_parse_sample_paths() {
    let sample = r#"
[stepsoundpack]
2
step1.wav
step2.wav

0 (Tür VV)
[pathpnt]
0.85
4.12
0.35

1 (Kasse)
[pathpnt]
0.10
3.50
0.40

[pathlink]
0
1

[next_stepsound]
0

[next_roomheight]
2.15
"#;
    let paths = parse_paths(sample).unwrap();
    assert_eq!(paths.sound_packs.len(), 1);
    assert_eq!(paths.sound_packs[0].sounds.len(), 2);
    assert_eq!(paths.points.len(), 2);
    assert_eq!(paths.points[0].index, 0);
    assert_eq!(paths.points[0].position, Vec3::new(0.85, 4.12, 0.35));
    assert_eq!(paths.points[1].index, 1);
    assert_eq!(paths.links.len(), 1);
    assert_eq!(
        paths.links[0],
        PathLink {
            from: 0,
            to: 1,
            one_way: false
        }
    );
    assert_eq!(paths.next_stepsound, Some(0));
    assert_eq!(paths.next_roomheight, Some(2.15));
}

#[test]
fn a_oneway_link_is_marked_and_keeps_its_place_in_the_list() {
    let paths = parse_paths(
        "[pathpnt]\n0\n0\n0\n[pathpnt]\n0\n1\n0\n[pathpnt]\n0\n2\n0\n[pathlink_oneway]\n1\n0\n[pathlink]\n1\n2\n",
    )
    .unwrap();
    let one_way: Vec<_> = paths
        .links
        .iter()
        .map(|l| (l.from, l.to, l.one_way))
        .collect();
    assert_eq!(one_way, vec![(1, 0, true), (1, 2, false)]);
}

#[test]
fn test_insufficient_coords_fails() {
    let sample = r#"
[pathpnt]
0.5
1.0
"#;
    assert!(matches!(
        parse_paths(sample),
        Err(PathsParseError::MissingParameters { .. })
    ));
}
