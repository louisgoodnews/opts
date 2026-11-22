//! Truncation tests for the untrusted text `.x`, `.bus`, `.sli` and `.hof` readers (ISS-005, ISS-006).
//!
//! Every prefix of a small in-code sample must return `Ok` or `Err` and must not panic. The
//! full sample must parse. No game file is read.

use opts_omsi::formats::scenery::sli::SplineProfile;
use opts_omsi::formats::vehicles::bus::BusManifest;
use opts_omsi::formats::vehicles::hof::HofFile;
use opts_omsi::formats::x::parse_x;

const X: &str = "xof 0303txt 0032\n\
Frame Root {\n  Mesh Quad {\n    4;\n    0.0; 0.0; 0.0;,\n    2.0; 0.0; 0.0;,\n\
    2.0; 2.0; 0.0;,\n    0.0; 2.0; 0.0;;\n    1;\n    4; 0, 1, 2, 3;;\n  }\n}\n";

const BUS: &str = "[friendlyname]\nMAN\nNL202\nBVG Standard\n\n[description]\nStandard\n[end]\n\
[coupling_front]\n0\n4.5\n0.3\n\n[coupling_front_character]\n52.5\n-20\n20\n1\n";

const SLI: &str = "[profilepnt]\n-2\n0.1\n0.5\n0.2\n[profilepnt]\n2\n0.1\n0.5\n0.2\n";

const HOF: &str = "[name]\nDemo Depot\n\n[global_strings]\n3\nDemo_Announcements\nDemo_Rollband\n\
Demo_Sidesign\n\n[addbusstop_list]\nHbf\tHBF\tHauptbahnhof\t\tHauptbahnhof\n[end]\n";

/// Calls `parse` on every prefix of `sample` (byte-wise, so cuts may split a UTF-8 sequence).
/// A panic fails the test; an error or a value is accepted.
fn every_prefix_is_handled(sample: &[u8], parse: impl Fn(&[u8]) -> bool) {
    for end in 0..=sample.len() {
        let _ = parse(&sample[..end]);
    }
}

#[test]
fn bus_every_prefix_returns_without_panic() {
    assert!(
        BusManifest::parse_bytes(BUS.as_bytes()).is_ok(),
        "full sample must parse"
    );
    every_prefix_is_handled(BUS.as_bytes(), |b| BusManifest::parse_bytes(b).is_ok());
}

#[test]
fn sli_every_prefix_returns_without_panic() {
    assert!(
        SplineProfile::parse(SLI.as_bytes()).is_ok(),
        "full sample must parse"
    );
    every_prefix_is_handled(SLI.as_bytes(), |b| SplineProfile::parse(b).is_ok());
}

#[test]
fn hof_every_prefix_returns_without_panic() {
    assert!(
        HofFile::parse_bytes(HOF.as_bytes()).is_ok(),
        "full sample must parse"
    );
    every_prefix_is_handled(HOF.as_bytes(), |b| HofFile::parse_bytes(b).is_ok());
}

#[test]
fn x_every_prefix_returns_without_panic() {
    assert!(parse_x(X.as_bytes()).is_ok(), "full sample must parse");
    every_prefix_is_handled(X.as_bytes(), |b| parse_x(b).is_ok());
}
