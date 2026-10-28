//! Reference and candidate path extraction for scenery objects and splines.

use super::*;

/// File-entity categories: `(root folder, extensions, category)`.
pub(super) const FILE_ENTITIES: &[(&str, &[&str], &str)] = &[
    ("Sceneryobjects", &["sco"], "sceneryobject"),
    ("Splines", &["sli"], "spline"),
    ("Fonts", &["oft"], "font"),
    ("Trains", &["zug"], "train_consist"),
    ("Situations", &["osn"], "situation"),
    ("Weather", &["owt"], "weather"),
    ("option_presets", &["oop"], "option_preset"),
    ("Languages", &["olf"], "language"),
    ("Inputs", &["kyb"], "input_map"),
    ("GUI", &["bmp"], "gui_skin"),
    ("Drivers", &["odr"], "driver"),
    ("Humans", &["hum"], "human"),
    ("TicketPacks", &["otp"], "ticketpack"),
];

/// Candidate locations of a texture named by an object in `dir`: the object's own `texture/`
/// folder, the object folder, then the global `Texture/` folder; each tried with the exact name
/// and then with the other image extensions (material names often omit or mis-state them).
pub(super) fn texture_candidates(dir: &str, name: &str) -> Vec<String> {
    let n = norm(name);
    let stem = n
        .rsplit_once('.')
        .map_or(n.as_str(), |(s, _)| s)
        .to_string();
    let bases = [
        format!("{dir}/texture"),
        dir.to_string(),
        "Texture".to_string(),
    ];
    let mut out = Vec::new();
    for base in &bases {
        out.push(norm(&format!("{base}/{n}")));
    }
    for base in &bases {
        for ext in ["dds", "bmp", "tga", "png", "jpg"] {
            out.push(norm(&format!("{base}/{stem}.{ext}")));
        }
    }
    out
}

pub(super) fn scenery_refs(
    dir: &str,
    sco: &SceneryObject,
) -> Vec<(&'static str, String, Vec<String>)> {
    let mut refs = Vec::new();
    for mesh in &sco.meshes {
        refs.push((
            "model",
            mesh.file.clone(),
            vec![
                under(&format!("{dir}/model"), &mesh.file),
                under(dir, &mesh.file),
            ],
        ));
        for m in &mesh.materials {
            let named = [Some(&m.texture), m.light_map.as_ref(), m.night_map.as_ref()]
                .into_iter()
                .flatten()
                .chain(m.env_map.as_ref().map(|(t, _)| t));
            for t in named.filter(|t| !t.is_empty()) {
                refs.push(("texture", t.clone(), texture_candidates(dir, t)));
            }
        }
    }
    for c in &sco.collision_meshes {
        refs.push((
            "model",
            c.clone(),
            vec![under(&format!("{dir}/model"), c), under(dir, c)],
        ));
    }
    for h in &sco.spline_helpers {
        refs.push((
            "spline",
            h.profile_file.clone(),
            vec![norm(&h.profile_file), under(dir, &h.profile_file)],
        ));
    }
    for (role, list) in [
        ("script", &sco.scripts),
        ("script", &sco.varnamelists),
        ("script", &sco.stringvarnamelists),
        ("constfile", &sco.constfiles),
    ] {
        refs.extend(list.iter().map(|f| (role, f.clone(), vec![under(dir, f)])));
    }
    if let Some(s) = &sco.sound {
        refs.push(("sound", s.clone(), vec![under(dir, s)]));
    }
    if let Some(t) = &sco.tree {
        refs.push((
            "texture",
            t.texture.clone(),
            texture_candidates(dir, &t.texture),
        ));
    }
    refs
}

pub(super) fn spline_refs(
    dir: &str,
    sli: &SplineProfile,
) -> Vec<(&'static str, String, Vec<String>)> {
    sli.textures
        .iter()
        .filter(|t| !t.file.is_empty())
        .map(|t| ("texture", t.file.clone(), texture_candidates(dir, &t.file)))
        .collect()
}
