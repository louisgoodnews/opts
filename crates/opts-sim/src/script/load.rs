//! Builds a [`ScriptEngine`] from a script set: script, variable-list and constant files as
//! listed by a `.bus` or a `.sco` (`[script]`, `[varnamelist]`, `[stringvarnamelist]`,
//! `[constfile]`).

use std::path::PathBuf;

use opts_core::formats::constfile::ConstFile;
use opts_core::formats::scenery::SceneryObject;
use opts_core::formats::text::decode_bytes;
use opts_core::formats::varlist::VarList;

use super::compiler::{Compiler, Diagnostic, Severity};
use super::ScriptEngine;

/// The files of one script set, as written in the owning file (relative paths).
#[derive(Debug, Clone, Copy)]
pub struct ScriptFiles<'a> {
    /// `.osc` scripts, compiled in this order.
    pub scripts: &'a [String],
    /// Float variable lists.
    pub varnamelists: &'a [String],
    /// String variable lists.
    pub stringvarnamelists: &'a [String],
    /// Constant files.
    pub constfiles: &'a [String],
}

/// A loaded scenery script set.
pub struct SceneryScriptSet {
    /// The engine (`{init}` not run yet).
    pub engine: ScriptEngine,
    /// Missing files, compile problems and the undeclared-variable summary.
    pub diagnostics: Vec<Diagnostic>,
    /// String variable names in `[stringvarnamelist]` order: a placed object's string `i`
    /// goes into variable `string_names[i]`.
    pub string_names: Vec<String>,
}

fn warning(file: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        file: file.to_owned(),
        line: 0,
        message,
    }
}

/// Loads a script set. `resolve` maps a path as written to a file; unresolvable or unreadable
/// files become warnings and are skipped. Returns the engine, the diagnostics and the string
/// variable names in list order.
pub fn load_script_files(
    files: ScriptFiles<'_>,
    mut resolve: impl FnMut(&str) -> Option<PathBuf>,
) -> (ScriptEngine, Vec<Diagnostic>, Vec<String>) {
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
    for rel in files.scripts {
        if let Some((name, bytes)) = read(rel, &mut diags) {
            compiler.add_source(&name, &decode_bytes(&bytes));
        }
    }
    let (program, compile_diags) = compiler.finish();
    diags.extend(compile_diags);
    let mut engine = ScriptEngine::new(program);
    for rel in files.varnamelists {
        if let Some((_, bytes)) = read(rel, &mut diags) {
            engine.declare_variables(&VarList::parse(&bytes).names, false);
        }
    }
    let mut string_names = Vec::new();
    for rel in files.stringvarnamelists {
        if let Some((_, bytes)) = read(rel, &mut diags) {
            let names = VarList::parse(&bytes).names;
            engine.declare_variables(&names, true);
            string_names.extend(names);
        }
    }
    for rel in files.constfiles {
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
    (engine, diags, string_names)
}

/// Loads the scripts of a scenery object (`docs/src/architecture/scenery-runtime.md`). `resolve`
/// maps a path as written in the `.sco` (relative to its folder, `..` allowed) to a file.
pub fn load_scenery_scripts(
    sco: &SceneryObject,
    resolve: impl FnMut(&str) -> Option<PathBuf>,
) -> SceneryScriptSet {
    let (engine, diagnostics, string_names) = load_script_files(
        ScriptFiles {
            scripts: &sco.scripts,
            varnamelists: &sco.varnamelists,
            stringvarnamelists: &sco.stringvarnamelists,
            constfiles: &sco.constfiles,
        },
        resolve,
    );
    SceneryScriptSet {
        engine,
        diagnostics,
        string_names,
    }
}
