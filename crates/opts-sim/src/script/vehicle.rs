//! Builds a [`ScriptEngine`] from a vehicle's `.bus` script, variable and constant lists.

use std::path::PathBuf;

use opts_core::formats::constfile::ConstFile;
use opts_core::formats::text::decode_bytes;
use opts_core::formats::varlist::VarList;
use opts_core::formats::vehicles::BusManifest;

use super::compiler::{Compiler, Diagnostic, Severity};
use super::ScriptEngine;

fn warning(file: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        file: file.to_owned(),
        line: 0,
        message,
    }
}

/// Loads the scripts of `bus` (`[script]`, `[varnamelist]`, `[stringvarnamelist]`,
/// `[constfile]`). `resolve` maps a path as written in the `.bus` to a file; unresolvable or
/// unreadable files become warnings and are skipped.
///
/// Scripts compile in `.bus` order, so `{init}`/`{frame}` blocks run in that order. For a vehicle
/// with `[scriptshare]` the caller loads the leading unit's `BusManifest` instead.
pub fn load_vehicle_scripts(
    bus: &BusManifest,
    mut resolve: impl FnMut(&str) -> Option<PathBuf>,
) -> (ScriptEngine, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let mut read = |rel: &str, diags: &mut Vec<Diagnostic>| -> Option<(String, Vec<u8>)> {
        let Some(path) = resolve(rel) else {
            diags.push(warning(rel, "file not found".into()));
            return None;
        };
        match std::fs::read(&path) {
            Ok(bytes) => Some((path.to_string_lossy().into_owned(), bytes)),
            Err(e) => {
                diags.push(warning(rel, format!("unreadable: {e}")));
                None
            }
        }
    };
    let mut compiler = Compiler::new();
    for rel in &bus.scripts {
        if let Some((name, bytes)) = read(rel, &mut diags) {
            compiler.add_source(&name, &decode_bytes(&bytes));
        }
    }
    let (program, compile_diags) = compiler.finish();
    diags.extend(compile_diags);
    let mut engine = ScriptEngine::new(program);
    for rel in &bus.varnamelists {
        if let Some((_, bytes)) = read(rel, &mut diags) {
            engine.declare_variables(&VarList::parse(&bytes).names, false);
        }
    }
    for rel in &bus.stringvarnamelists {
        if let Some((_, bytes)) = read(rel, &mut diags) {
            engine.declare_variables(&VarList::parse(&bytes).names, true);
        }
    }
    for rel in &bus.constfiles {
        if let Some((name, bytes)) = read(rel, &mut diags) {
            let file = ConstFile::parse(&bytes);
            diags.extend(file.warnings.iter().map(|w| warning(&name, w.clone())));
            engine.add_constfile(&file);
        }
    }
    let undeclared = engine.undeclared_variables();
    if !undeclared.is_empty() {
        diags.push(warning(
            "",
            format!(
                "{} variables are used without a varlist declaration (predefined or undeclared)",
                undeclared.len()
            ),
        ));
    }
    (engine, diags)
}
