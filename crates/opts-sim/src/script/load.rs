//! Builds a [`ScriptEngine`] from a script set: script, variable-list and constant files as
//! listed by a `.bus` or a `.sco` (`[script]`, `[varnamelist]`, `[stringvarnamelist]`,
//! `[constfile]`). The loaders do no file I/O: the caller resolves and reads each file and
//! reports the outcome as a [`FileRead`].

use opts_core::formats::constfile::ConstFile;
use opts_core::formats::scenery::SceneryObject;
use opts_core::formats::text::decode_bytes;
use opts_core::formats::varlist::VarList;

use super::compiler::{Compiler, Diagnostic, Severity};
use super::ScriptEngine;

/// The outcome of the caller's attempt to read one file of a script set. The caller resolves the
/// path as written and reads the file; the loader turns a failure into a warning.
#[derive(Debug, Clone)]
pub enum FileRead {
    /// The file was read. `name` is the resolved path, used as the diagnostic file name.
    Loaded {
        /// The resolved path, as a diagnostic file name.
        name: String,
        /// The file contents.
        bytes: Vec<u8>,
    },
    /// The path does not resolve to a file.
    NotFound,
    /// The file was found but could not be read; the text is the error message.
    Unreadable(String),
}

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

/// Loads a script set. `fetch` maps a path as written to its [`FileRead`]; files that are not
/// found or unreadable become warnings and are skipped. Returns the engine, the diagnostics and
/// the string variable names in list order.
pub fn load_script_files(
    files: ScriptFiles<'_>,
    mut fetch: impl FnMut(&str) -> FileRead,
) -> (ScriptEngine, Vec<Diagnostic>, Vec<String>) {
    let mut diags = Vec::new();
    let mut read = |rel: &str, diags: &mut Vec<Diagnostic>| -> Option<(String, Vec<u8>)> {
        match fetch(rel) {
            FileRead::Loaded { name, bytes } => Some((name, bytes)),
            FileRead::NotFound => {
                diags.push(warning(rel, "file not found".into()));
                None
            }
            FileRead::Unreadable(e) => {
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

/// Loads the scripts of a scenery object (`docs/src/architecture/scenery-runtime.md`). `fetch`
/// maps a path as written in the `.sco` (relative to its folder, `..` allowed) to its
/// [`FileRead`].
pub fn load_scenery_scripts(
    sco: &SceneryObject,
    fetch: impl FnMut(&str) -> FileRead,
) -> SceneryScriptSet {
    let (engine, diagnostics, string_names) = load_script_files(
        ScriptFiles {
            scripts: &sco.scripts,
            varnamelists: &sco.varnamelists,
            stringvarnamelists: &sco.stringvarnamelists,
            constfiles: &sco.constfiles,
        },
        fetch,
    );
    SceneryScriptSet {
        engine,
        diagnostics,
        string_names,
    }
}
