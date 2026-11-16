//! Classification of vehicle names and folders.

use super::*;

/// Applies playability rules R1–R5 plus R9/R10 (`playability-rules.md`). A `.bus` that survives
/// them is playable.
///
/// R9: a vehicle without a complete driver workstation (a driver camera *and* `[drivpos]` in its
/// passenger cabin) cannot be driven (bicycles, fire engines, lorries delivered as `.bus`).
/// R11: a rail vehicle without a `[sound]` block is a passenger-car variant (the NF6D `B` with
/// the dummy plate model and the `Redux` scripts), not a head; the head is `B_User`.
/// R10: vehicles that live in a folder named for AI/KI traffic are traffic, not player vehicles.
pub(in crate::extract) fn classify_playable(
    ext: &str,
    rel_path: &str,
    script_share: bool,
    inbound_trailer_target: bool,
    has_full_cab: bool,
    silent_rail: bool,
) -> bool {
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    let r1 = ext == "ovh";
    let r2 = script_share;
    let r3 = inbound_trailer_target && !has_full_cab;
    let r4_r5 = is_trailer_name(name) || is_ai_name(name);
    let r9 = !has_full_cab;
    let r10 = in_ai_folder(rel_path);
    !(r1 || r2 || r3 || r4_r5 || r9 || r10 || silent_rail)
}

/// Whether a directory component of `rel_path` contains the token `ai` or `ki` (split on
/// non-alphanumerics, case-insensitive), e.g. `HOH_KI_Autos`, `AI-Traffic`.
pub(in crate::extract) fn in_ai_folder(rel_path: &str) -> bool {
    let mut parts: Vec<&str> = rel_path.split('/').collect();
    parts.pop();
    parts.iter().any(|dir| {
        dir.split(|c: char| !c.is_alphanumeric())
            .any(|t| t.eq_ignore_ascii_case("ai") || t.eq_ignore_ascii_case("ki"))
    })
}

pub(in crate::extract) fn is_trailer_name(n: &str) -> bool {
    let n = n.to_lowercase();
    ["_trail", "_trailer", "_heck", "_nachl"]
        .iter()
        .any(|s| stem(&n).ends_with(s))
}

pub(in crate::extract) fn is_ai_name(n: &str) -> bool {
    // `ai` / `ki` as a whole token anywhere in the stem: `AI_mb_w123`, `MAN_SD84_KI`,
    // `MB_KI_C2_E5_Solo`, `HHStadtbus_VHH_AI_OMSI2`; never inside a word (`Kaiser`).
    let s = stem(&n.to_lowercase());
    s.split(|c: char| !c.is_alphanumeric())
        .any(|t| t == "ai" || t == "ki")
}
