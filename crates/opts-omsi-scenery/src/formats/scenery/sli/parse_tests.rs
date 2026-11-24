//! Unit tests of the block reader on representative `.sli` text: decimal commas, CRLF and BOM
//! input, and malformed lines inside a block (the corpus cases that used to reject a whole file).

use super::*;

const POINTS: &str = "[profilepnt]\n-2\n0.1\n0.5\n0.2\n[profilepnt]\n2\n0.1\n0.5\n0.2\n";

fn strip_points(sli: &SplineProfile) -> Vec<(f32, f32)> {
    sli.graphical_profiles
        .iter()
        .flat_map(|g| g.points.iter().map(|p| (p.x, p.z)))
        .collect()
}

#[test]
fn decimal_commas_crlf_and_a_bom_parse_like_plain_text() {
    let text = "\u{feff}[length]\r\n10\r\n[texture]\r\nroad.bmp\r\n[profile]\r\n0\r\n\
                [profilepnt]\r\n-2,5\r\n0,1\r\n0,5\r\n0,2\r\n[profilepnt]\r\n2,5\r\n0,1\r\n0,5\r\n0,2\r\n";
    let sli = SplineProfile::parse(text.as_bytes()).expect("parses");
    assert_eq!(sli.length, Some(10.0));
    assert_eq!(strip_points(&sli), vec![(-2.5, 0.1), (2.5, 0.1)]);
}

#[test]
fn a_stray_line_in_a_path_block_drops_only_that_block() {
    let text =
        format!("[profile]\n0\n{POINTS}[path]\n1.5\n-2\n0.1\n3\n1\n[path]\n0\n2.5\n0.1\n5\n0\n");
    let sli = SplineProfile::from_text(&text).expect("parses");
    assert_eq!(strip_points(&sli).len(), 2, "profile points kept");
    assert_eq!(
        sli.paths.len(),
        1,
        "the malformed path is skipped, the next kept"
    );
    assert_eq!(sli.paths[0].offset_x, 2.5);
}

#[test]
fn a_tutorial_fragment_in_a_matl_block_does_not_reject_the_file() {
    let text = format!("[texture]\nroad.bmp\n[matl]\n== START ==\n[profile]\n0\n{POINTS}");
    let sli = SplineProfile::from_text(&text).expect("parses");
    assert_eq!(sli.textures.len(), 1);
    assert_eq!(strip_points(&sli).len(), 2);
}

#[test]
fn a_malformed_profile_index_keeps_its_points_on_texture_zero() {
    let text = format!("[profile]\n== START ==\n{POINTS}");
    let sli = SplineProfile::from_text(&text).expect("parses");
    assert_eq!(sli.graphical_profiles.len(), 1);
    assert_eq!(sli.graphical_profiles[0].texture_index, 0);
    assert_eq!(sli.graphical_profiles[0].points.len(), 2);
}
