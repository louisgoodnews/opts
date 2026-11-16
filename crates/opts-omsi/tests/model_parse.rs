//! Integration and fuzz tests for vehicle model configuration parsing (`model.cfg`).

use opts_omsi::formats::vehicles::model::{
    parse_model, ModelConfig, ModelParseError, ViewpointFlags,
};

const SAMPLE_BUS_MODEL: &str = include_str!("model_parse/sample_bus.cfg");

#[path = "model_parse/errors.rs"]
mod errors;
#[path = "model_parse/keywords.rs"]
mod keywords;
#[path = "model_parse/structure.rs"]
mod structure;
