//! Builds a [`ScriptEngine`] from a vehicle's `.bus` script, variable and constant lists.

use opts_core::formats::vehicles::BusManifest;

use super::compiler::Diagnostic;
use super::load::{load_script_files, FileRead, ScriptFiles};
use super::ScriptEngine;

/// Loads the scripts of `bus` (`[script]`, `[varnamelist]`, `[stringvarnamelist]`,
/// `[constfile]`). `fetch` maps a path as written in the `.bus` to its [`FileRead`]; files that
/// are not found or unreadable become warnings and are skipped.
///
/// Scripts compile in `.bus` order, so `{init}`/`{frame}` blocks run in that order. For a vehicle
/// with `[scriptshare]` the caller loads the leading unit's `BusManifest` instead.
pub fn load_vehicle_scripts(
    bus: &BusManifest,
    fetch: impl FnMut(&str) -> FileRead,
) -> (ScriptEngine, Vec<Diagnostic>) {
    let (engine, diags, _) = load_script_files(
        ScriptFiles {
            scripts: &bus.scripts,
            varnamelists: &bus.varnamelists,
            stringvarnamelists: &bus.stringvarnamelists,
            constfiles: &bus.constfiles,
        },
        fetch,
    );
    (engine, diags)
}
