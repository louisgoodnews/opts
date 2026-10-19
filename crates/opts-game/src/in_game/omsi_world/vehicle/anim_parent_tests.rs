//! Tests: anim_parent_tests.

use super::skins::{forms_cycle, referenced_idents};
use super::*;
use opts_core::formats::vehicles::model::ModelConfig;

fn meshes(text: &str) -> Vec<opts_core::formats::vehicles::model::ModelMesh> {
    let config = ModelConfig::parse(text).expect("model");
    config.lods.into_iter().next().expect("lod").meshes
}

/// The cockpit shape of the Hamburg e-bus: a dashboard base that tilts, with switches hung on it.
const COCKPIT: &str = "[mesh]\nbase.o3d\n[mesh_ident]\nCP_Basis\n\n\
[mesh]\nswitch_a.o3d\n[animparent]\ncp_basis\n\n\
[mesh]\nswitch_b.o3d\n[animparent]\nunknown_parent\n\n\
[mesh]\nflap.o3d\n[mesh_ident]\nflap\n[animparent]\nhandle\n\n\
[mesh]\nhandle.o3d\n[mesh_ident]\nhandle\n[animparent]\nflap\n";

#[test]
fn only_declared_and_named_idents_get_a_pivot() {
    let m = meshes(COCKPIT);
    let ids: Vec<_> = referenced_idents(&m).into_iter().collect();
    assert_eq!(ids, ["cp_basis", "flap", "handle"]);
}

#[test]
fn a_loop_of_parents_is_cut() {
    let m = meshes(COCKPIT);
    assert!(forms_cycle(&m, Some("flap"), Some("handle")));
    assert!(forms_cycle(&m, Some("handle"), Some("flap")));
    assert!(forms_cycle(&m, Some("x"), Some("x")), "own parent");
    assert!(
        !forms_cycle(&m, None, Some("cp_basis")),
        "a leaf has no ident"
    );
    assert!(!forms_cycle(&m, Some("switch"), Some("CP_BASIS")));
}
