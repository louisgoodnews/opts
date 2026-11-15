//! The callbacks of OMSI's lists with their stack signatures, and lookup by name.

use super::{CallbackSignature, Neutral};

const fn sig(
    name: &'static str,
    float_args: u8,
    string_args: u8,
    float_results: u8,
    string_results: u8,
    neutral: Neutral,
    confident: bool,
) -> CallbackSignature {
    CallbackSignature {
        name,
        float_args,
        string_args,
        float_results,
        string_results,
        neutral,
        confident,
    }
}

/// Every callback of OMSI's lists (`callbacklist_roadvehicle.txt` 25, `callbacklist_scripttex.txt`
/// 17, `callbacklist_scenobj.txt` 3), with the signature of the callbacks reference. The three
/// script-texture calls no installed script uses and the three scenery callbacks carry assumed
/// signatures (`confident == false`).
pub const CALLBACKS: &[CallbackSignature] = &[
    // Route and timetable (.hof tables, active timetable)
    sig("getrouteindex", 1, 0, 1, 0, Neutral::NotFound, true),
    sig("getbusstopstring", 2, 0, 0, 1, Neutral::Zero, true),
    sig("getterminusstring", 2, 0, 0, 1, Neutral::Zero, true),
    sig("getrouteterminusindex", 1, 0, 1, 0, Neutral::NotFound, true),
    sig("getterminusindex", 1, 0, 1, 0, Neutral::NotFound, true),
    sig("getterminuscode", 1, 0, 1, 0, Neutral::NotFound, true),
    sig("getbusstopcount", 1, 0, 1, 0, Neutral::Zero, true),
    sig("getroutebusstopident", 2, 0, 0, 1, Neutral::Zero, true),
    sig("getbusstopindex", 0, 1, 1, 0, Neutral::NotFound, true),
    sig("getdepotstringglobal", 1, 0, 0, 1, Neutral::Zero, true),
    sig("getttbusstopcount", 0, 0, 1, 0, Neutral::Zero, true),
    sig("getttbusstopindex", 0, 0, 1, 0, Neutral::Zero, true),
    sig("getttterminusindex", 0, 0, 1, 0, Neutral::NotFound, true),
    sig("getttlinestring", 0, 0, 0, 1, Neutral::Zero, true),
    sig("getttdelay", 0, 0, 1, 0, Neutral::Zero, true),
    sig("getttbusstopname", 1, 0, 0, 1, Neutral::Zero, true),
    sig("getttbusstopdep", 1, 0, 1, 0, Neutral::Zero, true),
    sig("getttbusstoparr", 1, 0, 1, 0, Neutral::Zero, true),
    // Tickets and cash
    sig("getticketname", 1, 0, 0, 1, Neutral::Zero, true),
    sig("getticketvalue", 1, 0, 1, 0, Neutral::Zero, true),
    sig("givechangecoin", 1, 0, 0, 0, Neutral::Zero, true),
    // Passengers, randomness, geometry
    sig("gethumancountonpathlink", 1, 0, 1, 0, Neutral::Zero, true),
    sig("gethumancountonseat", 1, 0, 1, 0, Neutral::Zero, true),
    sig("nrspecrandom", 1, 0, 1, 0, Neutral::Zero, true),
    sig("getheightabovepoint", 3, 0, 1, 0, Neutral::Zero, false),
    // Script textures
    sig("stnewtex", 1, 0, 0, 0, Neutral::Zero, true),
    sig("stlock", 1, 0, 0, 0, Neutral::Zero, true),
    sig("stunlock", 1, 0, 0, 0, Neutral::Zero, true),
    sig("stfilter", 1, 0, 0, 0, Neutral::Zero, true),
    sig("stsetcolor", 5, 0, 0, 0, Neutral::Zero, false),
    sig("stdrawpixel", 3, 0, 0, 0, Neutral::Zero, false),
    sig("stdrawrect", 5, 0, 0, 0, Neutral::Zero, true),
    sig("sttextout", 6, 1, 0, 0, Neutral::Zero, false),
    sig("getfontindex", 0, 1, 1, 0, Neutral::Zero, true),
    sig("textlength", 1, 1, 1, 0, Neutral::Zero, true),
    sig("streadpixel", 3, 0, 0, 0, Neutral::Zero, false),
    sig("stcopycolor", 1, 0, 0, 0, Neutral::Zero, false),
    sig("stloadtex", 1, 1, 0, 0, Neutral::Zero, false),
    sig("stgetr", 1, 0, 1, 0, Neutral::Zero, true),
    sig("stgetg", 1, 0, 1, 0, Neutral::Zero, true),
    sig("stgetb", 1, 0, 1, 0, Neutral::Zero, true),
    sig("stgeta", 1, 0, 1, 0, Neutral::Zero, false),
    // Scenery objects (bus-stop displays)
    sig("getarrbusline", 1, 0, 0, 1, Neutral::Zero, false),
    sig("getarrbusterminus", 1, 0, 0, 1, Neutral::Zero, false),
    sig("getarrbustimediff", 1, 0, 1, 0, Neutral::Zero, false),
];

/// The signature of callback `name` (any case), if it is one of OMSI's.
#[must_use]
pub fn signature(name: &str) -> Option<&'static CallbackSignature> {
    let lower = name.to_ascii_lowercase();
    CALLBACKS.iter().find(|s| s.name == lower)
}
