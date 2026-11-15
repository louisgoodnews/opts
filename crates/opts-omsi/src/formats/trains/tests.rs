//! Parser tests with self-authored fixtures (no OMSI file is used).

use super::*;

mod editing;
mod encoding;
mod layout;
mod model;
mod validation;

fn parse(text: &str) -> Result<(TrainFormation, Vec<ZugNote>), ZugError> {
    TrainFormation::from_zug(text.as_bytes(), "t", "Trains\\t.zug")
}

fn ok(text: &str) -> TrainFormation {
    parse(text).expect("parses").0
}
