//! Detection for file entities (scenery objects, splines, humans, money).

use std::collections::HashSet;

use rayon::prelude::*;

use crate::scan::FileRec;

use super::refs::{scenery_refs, spline_refs, FILE_ENTITIES};
use super::{friendly_name, stem, EntityDraft, Info, SceneryObject, SplineProfile};

/// Constructs a basic draft with empty reference list.
pub(super) fn simple(
    category: &'static str,
    mode: &'static str,
    f: &FileRec,
    name: String,
) -> EntityDraft {
    EntityDraft {
        category,
        mode,
        name,
        marker: f.rel_path.clone(),
        dir: f.dir.clone(),
        is_playable: true,
        vehicle_class: None,
        alias_of_marker: None,
        mtime: f.mtime,
        info: Info::None,
        refs: Vec::new(),
    }
}

/// Detects money pack entities (one per currency folder).
pub(super) fn detect_money(files: &[FileRec]) -> Vec<EntityDraft> {
    let mut seen_money: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for f in files
        .iter()
        .filter(|f| !f.is_backup && f.ext == "cfg" && f.rel_path.starts_with("Money/"))
    {
        if f.rel_path.matches('/').count() == 2 && seen_money.insert(f.dir.clone()) {
            out.push(simple(
                "money",
                "folder",
                f,
                f.dir.rsplit('/').next().unwrap_or("").to_string(),
            ));
        }
    }
    out
}

/// One reference of a file entity: its role (e.g. `texture`), the referenced name and the
/// candidate paths it may resolve to.
type FileRef = (&'static str, String, Vec<String>);

/// Extracts entity name and outgoing references for a single file entity.
fn parse_file_entity_refs(
    cat: &'static str,
    bytes: Option<&[u8]>,
    f: &FileRec,
) -> (String, Vec<FileRef>) {
    let mut refs = Vec::new();
    let name = match (cat, bytes) {
        ("sceneryobject", Some(b)) => match SceneryObject::parse(b) {
            Ok(sco) => {
                refs = scenery_refs(&f.dir, &sco);
                sco.friendly_name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| stem(&f.name))
            }
            Err(_) => stem(&f.name),
        },
        ("spline", Some(b)) => {
            if let Ok(sli) = SplineProfile::parse(b) {
                refs = spline_refs(&f.dir, &sli);
            }
            stem(&f.name)
        }
        ("human", Some(b)) => friendly_name(b).unwrap_or_else(|| stem(&f.name)),
        _ => stem(&f.name),
    };
    (name, refs)
}

/// Detects individual file entities (scenery objects, splines, humans, textures, fonts).
pub(super) fn detect_file_entities(
    files: &[FileRec],
    read: &(dyn Fn(&str) -> Option<Vec<u8>> + Sync),
) -> Vec<EntityDraft> {
    files
        .par_iter()
        .filter(|f| !f.is_backup)
        .filter_map(|f| {
            let top = f.rel_path.split('/').next()?;
            let (_, _, cat) = FILE_ENTITIES.iter().find(|(root_dir, exts, _)| {
                top.eq_ignore_ascii_case(root_dir) && exts.contains(&f.ext.as_str())
            })?;
            let bytes = if matches!(*cat, "sceneryobject" | "spline" | "human") {
                read(&f.rel_path)
            } else {
                None
            };
            let (name, refs) = parse_file_entity_refs(cat, bytes.as_deref(), f);
            let mut draft = simple(cat, "file", f, name);
            draft.refs = refs;
            Some(draft)
        })
        .collect()
}
