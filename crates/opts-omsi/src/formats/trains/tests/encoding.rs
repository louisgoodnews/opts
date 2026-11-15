//! Unit tests of the train formation parser and the formation model.

use super::*;

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
