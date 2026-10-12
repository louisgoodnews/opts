//! Builds a [`ScriptEngine`] from a vehicle's `.bus` script, variable and constant lists.

use std::path::PathBuf;

use opts_core::formats::vehicles::BusManifest;

use super::compiler::Diagnostic;
use super::load::{load_script_files, ScriptFiles};
use super::ScriptEngine;

/// Loads the scripts of `bus` (`[script]`, `[varnamelist]`, `[stringvarnamelist]`,
/// `[constfile]`). `resolve` maps a path as written in the `.bus` to a file; unresolvable or
/// unreadable files become warnings and are skipped.
///
/// Scripts compile in `.bus` order, so `{init}`/`{frame}` blocks run in that order. For a vehicle
/// with `[scriptshare]` the caller loads the leading unit's `BusManifest` instead.
pub fn load_vehicle_scripts(
    bus: &BusManifest,
    resolve: impl FnMut(&str) -> Option<PathBuf>,
) -> (ScriptEngine, Vec<Diagnostic>) {
    let (engine, diags, _) = load_script_files(
        ScriptFiles {
            scripts: &bus.scripts,
            varnamelists: &bus.varnamelists,
            stringvarnamelists: &bus.stringvarnamelists,
            constfiles: &bus.constfiles,
        },
        resolve,
    );
    (engine, diags)
}
