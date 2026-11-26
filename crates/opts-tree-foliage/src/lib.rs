//! Foliage by texture name: which objects model a tree or bush as a mesh, and so sway in the
//! wind (M-0144 phase C).

use std::path::Path;

use opts_core::formats::scenery::SceneryObject;

/// Words of a foliage texture's name (lower case, `ae`/`oe`/`ue` for umlauts).
const FOLIAGE_WORDS: &[&str] = &[
    "laub",
    "leaf",
    "leaves",
    "blatt",
    "blaetter",
    "foliage",
    "nadel",
    "zweig",
    "branch",
    "baum",
    "baeume",
    "tree",
    "busch",
    "buesche",
    "bush",
    "strauch",
    "straeucher",
    "hecke",
    "hedge",
    "conifer",
    "tanne",
    "fichte",
    "kiefer",
    "pine",
    "birke",
    "eiche",
    "oak",
    "linde",
    "ahorn",
    "kastanie",
    "chestnut",
    "pappel",
    "weide",
    "palm",
    "palme",
    "gras",
    "grass",
];

/// The generic German words that may also end a compound (`Laubbaum`, `Kastanienbaum`,
/// `Ziergras`); the others only stand as words of their own (`SK_Bushokje` is a shelter, a
/// `Zifferblatt` a clock face).
const COMPOUND_TAILS: &[&str] = &[
    "baum",
    "baeume",
    "busch",
    "buesche",
    "strauch",
    "straeucher",
    "hecke",
    "laub",
    "gras",
];

/// Plural and inflection endings a word may carry (`Tannen`, `Birken`, `Zweige`, `leafs`).
const ENDINGS: &[&str] = &["", "s", "e", "n", "en", "es", "er"];

/// Whether `token` is a foliage word (optionally inflected), or ends in one that may close a
/// compound.
fn is_foliage_token(token: &str) -> bool {
    ENDINGS.iter().any(|end| {
        let Some(stem) = token.strip_suffix(end) else {
            return false;
        };
        FOLIAGE_WORDS.contains(&stem)
            || COMPOUND_TAILS
                .iter()
                .any(|w| stem.len() > w.len() && stem.ends_with(w))
    })
}

/// Whether a texture's name names foliage: one of its words (split at anything but letters)
/// is a foliage word.
pub fn is_foliage_texture(texture: &str) -> bool {
    let name = Path::new(texture.trim())
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue");
    name.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|t| !t.is_empty())
        .any(is_foliage_token)
}

/// Whether a placed object models a tree (or a bush) as a mesh: no `[tree]` billboard, and its
/// cut-out or blended materials (`[matl_alpha]` 1 or 2) all have foliage textures. Its foliage
/// then moves in the wind like the billboards (M-0144 phase C). The texture name is the only
/// hint OMSI's files give: a corpus scan finds 246 such objects next to 941 billboard trees, and
/// the word rule keeps railings (`Lichtentanne_Gelaender`) and signs out. A building with a bush
/// texture next to its window glass (`building401`) is no tree: its glass must not sway.
pub fn is_mesh_tree(sco: &SceneryObject) -> bool {
    if sco.tree.is_some() {
        return false;
    }
    let mut alpha = sco
        .meshes
        .iter()
        .filter(|m| !m.only_editor)
        .flat_map(|m| &m.materials)
        .filter(|t| matches!(t.alpha_mode, Some(1 | 2)))
        .peekable();
    alpha.peek().is_some() && alpha.all(|t| is_foliage_texture(&t.texture))
}
