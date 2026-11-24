//! Metadata and file entries of a `.bus` manifest: the names, the registration mode and the
//! referenced cfg, script and variable list files.

use super::super::blocks::extract_counted_list;
use super::super::*;
use crate::formats::text::OmsiBlock;

pub(super) fn handle_metadata(block: &OmsiBlock, m: &mut BusManifest) {
    match block.keyword.as_str() {
        "friendlyname" => {
            if block.parameters.len() >= 3 {
                m.friendly_name = Some(FriendlyName {
                    manufacturer: block.parameters[0].clone(),
                    model: block.parameters[1].clone(),
                    livery: block.parameters[2].clone(),
                });
            }
        }
        "description" => {
            for line in &block.parameters {
                if line == "[end]" {
                    break;
                }
                m.description.push(line.clone());
            }
        }
        "number" => {
            if let Some(first) = block.parameters.first() {
                m.number_file = Some(first.clone());
            }
        }
        "registration_list" => {
            if let Some(first) = block.parameters.first() {
                m.registration_mode = RegistrationMode::List(first.clone());
            }
        }
        "registration_automatic" => {
            if let Some(first) = block.parameters.first() {
                m.registration_mode = RegistrationMode::Automatic(first.clone());
            }
        }
        "registration_free" => {
            m.registration_mode = RegistrationMode::Free;
        }
        _ => {}
    }
}

pub(super) fn handle_files(block: &OmsiBlock, m: &mut BusManifest) {
    match block.keyword.as_str() {
        "sound" => {
            if let Some(first) = block.parameters.first() {
                m.sound_cfg = Some(first.clone());
            }
        }
        "sound_ai" => {
            if let Some(first) = block.parameters.first() {
                m.sound_ai_cfg = Some(first.clone());
            }
        }
        "model" => {
            if let Some(first) = block.parameters.first() {
                m.model_cfg = Some(first.clone());
            }
        }
        "paths" => {
            if let Some(first) = block.parameters.first() {
                m.paths_cfg = Some(first.clone());
            }
        }
        "passengercabin" => {
            if let Some(first) = block.parameters.first() {
                m.passengercabin_cfg = Some(first.clone());
            }
        }
        "varnamelist" => extract_counted_list(&block.parameters, &mut m.varnamelists),
        "stringvarnamelist" => extract_counted_list(&block.parameters, &mut m.stringvarnamelists),
        "script" => extract_counted_list(&block.parameters, &mut m.scripts),
        "constfile" => extract_counted_list(&block.parameters, &mut m.constfiles),
        "scriptshare" => m.script_share = true,
        _ => {}
    }
}
