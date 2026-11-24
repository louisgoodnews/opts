//! The four readers under test, each with its valid sample.

use opts_omsi::formats::scenery::sli::SplineProfile;
use opts_omsi::formats::vehicles::bus::BusManifest;
use opts_omsi::formats::vehicles::hof::HofFile;
use opts_omsi::formats::x::parse_x;

use super::samples;

pub struct Reader {
    pub name: &'static str,
    pub sample: &'static str,
    pub parse: fn(&[u8]) -> bool,
}

fn x_ok(bytes: &[u8]) -> bool {
    parse_x(bytes).is_ok()
}

fn bus_ok(bytes: &[u8]) -> bool {
    BusManifest::parse_bytes(bytes).is_ok()
}

fn sli_ok(bytes: &[u8]) -> bool {
    SplineProfile::parse(bytes).is_ok()
}

fn hof_ok(bytes: &[u8]) -> bool {
    HofFile::parse_bytes(bytes).is_ok()
}

pub const X_READER: Reader = Reader {
    name: "x",
    sample: samples::X,
    parse: x_ok,
};
pub const BUS_READER: Reader = Reader {
    name: "bus",
    sample: samples::BUS,
    parse: bus_ok,
};
pub const SLI_READER: Reader = Reader {
    name: "sli",
    sample: samples::SLI,
    parse: sli_ok,
};
pub const HOF_READER: Reader = Reader {
    name: "hof",
    sample: samples::HOF,
    parse: hof_ok,
};
