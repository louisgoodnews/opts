//! Tests of the relocation search.

use super::relocation::pick_relocated;

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn a_single_candidate_is_taken() {
    let f = v(&["Splines/Oberstedt_V2/Wilsede_by_Oberpfalz/8m_Standard3.sli"]);
    assert!(pick_relocated(&f, "Splines/Oberpfalz 3D/Wilsede").is_some());
}

#[test]
fn equal_trailing_folders_win() {
    let f = v(&[
        "Sceneryobjects/ADDON_Bad_Huegelsdorf/Odanrebode/Blumenkasten1.sco",
        "Sceneryobjects/St-Servan_Additionnel/Tettau/Blumenkasten/Blumenkasten1.sco",
        "Sceneryobjects/Oberpfalz 3D/Krummenaab2/Blumenkasten1.sco",
    ]);
    let got = pick_relocated(&f, "Sceneryobjects/Tettau/Blumenkasten").unwrap();
    assert!(got.contains("St-Servan_Additionnel/Tettau"));
}

#[test]
fn shared_name_tokens_decide_when_no_folder_matches() {
    let f = v(&[
        "Splines/VKL´s Splines/Zaun.sli",
        "Splines/Altenessen-Splines/Zaun.sli",
    ]);
    let got = pick_relocated(&f, "Splines/Oberpfalz 3D/LVB_VKL_Splines_Private_Changes").unwrap();
    assert!(got.contains("VKL"));
}

#[test]
fn ambiguous_candidates_are_not_guessed() {
    let f = v(&["Splines/A/Zaun.sli", "Splines/B/Zaun.sli"]);
    assert!(pick_relocated(&f, "Splines/C").is_none());
}
