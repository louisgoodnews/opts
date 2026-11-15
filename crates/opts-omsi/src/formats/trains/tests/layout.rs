//! Unit tests of the train formation parser and the formation model.

use super::*;

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
