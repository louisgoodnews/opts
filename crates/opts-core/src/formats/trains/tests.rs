//! Parser tests with self-authored fixtures (no OMSI file is used).

use super::*;

fn parse(text: &str) -> Result<(TrainFormation, Vec<ZugNote>), ZugError> {
    TrainFormation::from_zug(text.as_bytes(), "t", "Trains\\t.zug")
}

fn ok(text: &str) -> TrainFormation {
    parse(text).expect("parses").0
}

#[test]
fn a_regular_file_has_one_segment_per_pair_in_order() {
    let f = ok(
        "vehicles\\A\\loco.ovh\r\n0\r\nvehicles\\A\\wagon.ovh\r\n1\r\nvehicles\\A\\wagon.ovh\r\n0",
    );
    assert_eq!(f.len(), 3);
    assert_eq!(
        f.segments[0].vehicle,
        VehicleRef::File("vehicles\\A\\loco.ovh".into())
    );
    assert_eq!(f.segments[1].orientation, Orientation::Backward);
    assert_eq!(f.segments[2].orientation, Orientation::Forward);
    assert_eq!(f.source, FormationSource::ZugFile("Trains\\t.zug".into()));
    assert_eq!(f.name, "t");
}

#[test]
fn line_ending_and_final_break_do_not_matter() {
    let crlf = ok("a.ovh\r\n0\r\nb.ovh\r\n1\r\n");
    let lf = ok("a.ovh\n0\nb.ovh\n1");
    let blank_tail = ok("a.ovh\r\n0\r\nb.ovh\r\n1\r\n\r\n\r\n");
    assert_eq!(crlf, lf);
    assert_eq!(crlf, blank_tail);
}

#[test]
fn a_bare_name_is_a_group_and_a_path_or_extension_is_a_file() {
    let f = ok("M-Train Cars\r\n0\r\nvehicles\\x\\car.OVH\r\n1\r\ncar.bus\r\n0\r\nfolder/car\r\n0");
    assert!(matches!(&f.segments[0].vehicle, VehicleRef::Group(g) if g == "M-Train Cars"));
    assert!(matches!(f.segments[1].vehicle, VehicleRef::File(_)));
    assert!(matches!(f.segments[2].vehicle, VehicleRef::File(_)));
    assert!(matches!(f.segments[3].vehicle, VehicleRef::File(_)));
    assert!(f.has_groups());
    assert_eq!(f.file_refs().count(), 3);
}

#[test]
fn an_odd_last_line_is_dropped_with_a_note() {
    let (f, notes) = parse("a.ovh\r\n0\r\nb.ovh").expect("parses");
    assert_eq!(f.len(), 1);
    assert_eq!(notes, vec![ZugNote::DroppedLastLine("b.ovh".into())]);
}

#[test]
fn orientation_must_be_zero_or_one_and_the_error_names_the_line() {
    assert_eq!(
        parse("a.ovh\r\n0\r\nb.ovh\r\n2"),
        Err(ZugError::BadOrientation {
            line: 4,
            value: "2".into()
        })
    );
    assert_eq!(
        parse("a.ovh\r\nyes"),
        Err(ZugError::BadOrientation {
            line: 2,
            value: "yes".into()
        })
    );
}

#[test]
fn empty_and_blank_files_are_refused() {
    assert_eq!(parse(""), Err(ZugError::Empty));
    assert_eq!(parse("\r\n\r\n"), Err(ZugError::Empty));
    assert_eq!(
        parse("only.ovh"),
        Err(ZugError::Empty),
        "one line is no pair"
    );
}

#[test]
fn a_blank_vehicle_line_is_an_error() {
    assert_eq!(parse("\r\n0"), Err(ZugError::EmptyVehicle { line: 1 }));
}

#[test]
fn script_text_with_the_wrong_extension_is_refused_by_structure() {
    assert_eq!(
        parse("'#################\r\n'# MAIN SCRIPT #\r\n{init}\r\n(M.L.x)"),
        Err(ZugError::NotAConsist { line: 1 })
    );
    assert_eq!(
        parse("a.ovh\r\n0\r\n{init}\r\n0"),
        Err(ZugError::NotAConsist { line: 3 })
    );
}

#[test]
fn windows_1252_paths_decode() {
    // `MÁV` with Á as the single byte 0xC1.
    let bytes = b"vehicles\\M\xC1V\\Flirt_a.ovh\r\n0\r\nvehicles\\M\xC1V\\Flirt_b.ovh\r\n0";
    let (f, _) = TrainFormation::from_zug(bytes, "f", "Trains\\f.zug").expect("parses");
    assert_eq!(f.segments[0].vehicle.as_str(), "vehicles\\MÁV\\Flirt_a.ovh");
}

#[test]
fn a_utf8_bom_and_utf16_are_accepted() {
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice(b"a.ovh\r\n0\r\nb.ovh\r\n1");
    assert_eq!(
        TrainFormation::from_zug(&bom, "b", "p")
            .expect("bom")
            .0
            .len(),
        2
    );
    let mut u16le = vec![0xFF, 0xFE];
    for u in "a.ovh\r\n0\r\nb.ovh\r\n1".encode_utf16() {
        u16le.extend_from_slice(&u.to_le_bytes());
    }
    assert_eq!(
        TrainFormation::from_zug(&u16le, "u", "p")
            .expect("utf16")
            .0
            .len(),
        2
    );
}

#[test]
fn more_than_the_limit_is_refused() {
    let many: String = (0..300).map(|_| "a.ovh\r\n0\r\n").collect();
    assert_eq!(parse(&many), Err(ZugError::TooLong));
    let max: String = (0..256).map(|_| "a.ovh\r\n0\r\n").collect();
    assert_eq!(ok(&max).len(), 256);
}

#[test]
fn text_round_trips_and_the_model_serialises() {
    let f = ok("a.ovh\r\n0\r\nb.ovh\r\n1\r\nGroup X\r\n0");
    let (again, _) =
        TrainFormation::from_zug(f.to_zug_text().as_bytes(), "t", "Trains\\t.zug").expect("again");
    assert_eq!(again, f);
    let json = serde_json::to_string(&f).expect("json");
    let back: TrainFormation = serde_json::from_str(&json).expect("from json");
    assert_eq!(back, f);
}

#[test]
fn orientation_helpers() {
    assert_eq!(Orientation::Forward.zug_value(), 0);
    assert_eq!(Orientation::Backward.zug_value(), 1);
    assert_eq!(Orientation::Forward.flipped(), Orientation::Backward);
    assert_eq!(Orientation::default(), Orientation::Forward);
}

#[test]
fn a_custom_formation_starts_empty() {
    let f = TrainFormation::custom("Mine");
    assert!(f.is_empty());
    assert_eq!(f.source, FormationSource::Custom);
}
