//! Parsing of `[NightMapMode]` in scenery objects (M-0120): the mode number is read from the line
//! after the keyword, and a keyword without a number leaves the mode unset. Self-written fixtures.

use opts_omsi::formats::scenery::SceneryObject;

#[test]
fn night_map_mode_is_read_from_the_keyword_line() {
    let sco = SceneryObject::from_text("[NightMapMode]\n2\n[mesh]\nlamp.o3d\n").expect("parses");
    assert_eq!(sco.night_map_mode, Some(2));
}

#[test]
fn night_map_mode_is_unset_when_the_object_has_no_keyword() {
    let sco = SceneryObject::from_text("[mesh]\nlamp.o3d\n").expect("parses");
    assert_eq!(sco.night_map_mode, None);
}
