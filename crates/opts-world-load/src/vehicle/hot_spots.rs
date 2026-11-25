//! Which `[mouseevent]` a vehicle mesh answers (M-0141).
//!
//! A mesh answers its own `[mouseevent]`. A mesh without one that hangs (`[animparent]`) on a
//! hot-spot whose own `[newanim]` is driven by the hot-spot's trigger variable answers that
//! hot-spot too: it is a piece of the moving control itself. The hinged windows of the Valiant
//! Citybus family carry the `[mouseevent]` (`cp_klappfenster_UL1`, animated by
//! `cp_klappfenster_UL1`) on the thin frame (`…VITRES_MOV_MAT_F1C.o3d`) and hang the glass
//! (`…VITRES_MOV_TRANS_F1E/F1I.o3d`) on it: only a frame line a few pixels wide reacted to the
//! mouse, a click on the pane did nothing. ❓ OMSI's own rule is not documented; the condition
//! keeps dashboards that move as a whole (`actia_move`, `vdv_move`, whose displays hang on the
//! dragged base but are driven by other variables) from turning their displays into drag handles.
//! A corpus scan of the installed models finds 2,839 such children in 197 model files: hinged and
//! sliding windows, the Munich Lion's City door flaps, an O550 speedometer needle.

use opts_core::formats::vehicles::model::ModelMesh;

/// The trigger a click on `meshes[index]` raises: its own `[mouseevent]`, else the one of the
/// nearest `[animparent]` ancestor with a `[mouseevent]` when that ancestor is animated by the
/// trigger's variable (see the module documentation).
pub(super) fn hot_spot_of(meshes: &[ModelMesh], index: usize) -> Option<&str> {
    let mesh = meshes.get(index)?;
    if let Some(own) = event(mesh) {
        return Some(own);
    }
    let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
    let mut at = mesh.animparent.as_deref()?;
    // At most one step per mesh: a loop of parents ends the walk.
    for _ in 0..meshes.len() {
        let parent = meshes
            .iter()
            .find(|m| m.mesh_ident.as_deref().is_some_and(|i| same(i, at)))?;
        if let Some(trigger) = event(parent) {
            let moves_with_it = parent
                .anims
                .iter()
                .filter_map(|a| a.driver_variable())
                .any(|v| same(v, trigger));
            return moves_with_it.then_some(trigger);
        }
        at = parent.animparent.as_deref()?;
    }
    None
}

/// A mesh's own `[mouseevent]`, trimmed; `None` when it has none or it is blank.
fn event(mesh: &ModelMesh) -> Option<&str> {
    mesh.mouseevent
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_core::formats::vehicles::model::ModelConfig;

    fn meshes(text: &str) -> Vec<ModelMesh> {
        let config = ModelConfig::parse(text).expect("model");
        config.lods.into_iter().next().expect("lod").meshes
    }

    /// The Valiant's hinged window: the frame carries the hot-spot and turns with its variable,
    /// the glass hangs on it; a second glass hangs on the first.
    const WINDOW: &str = "[mesh]\nframe.o3d\n[mesh_ident]\nf1\n[mouseevent]\ncp_klappfenster_UL1\n\
[newanim]\norigin_from_mesh\nanim_rot\ncp_klappfenster_UL1\n30\n\n\
[mesh]\nglass_ext.o3d\n[mesh_ident]\nf1e\n[animparent]\nF1\n\n\
[mesh]\nglass_int.o3d\n[animparent]\nf1e\n\n\
[mesh]\nhandle.o3d\n[animparent]\nf1\n[mouseevent]\nhandle_click\n";

    #[test]
    fn the_glass_of_a_hinged_window_answers_its_frame() {
        let m = meshes(WINDOW);
        assert_eq!(hot_spot_of(&m, 0), Some("cp_klappfenster_UL1"), "the frame");
        assert_eq!(hot_spot_of(&m, 1), Some("cp_klappfenster_UL1"), "the glass");
        assert_eq!(
            hot_spot_of(&m, 2),
            Some("cp_klappfenster_UL1"),
            "glass on glass"
        );
        assert_eq!(
            hot_spot_of(&m, 3),
            Some("handle_click"),
            "an own event wins"
        );
        assert_eq!(hot_spot_of(&m, 4), None, "out of range");
    }

    /// A dashboard dragged as a whole: its displays hang on the base, but the base turns with a
    /// variable of its own, so the displays stay plain meshes.
    const DASHBOARD: &str = "[mesh]\nbase.o3d\n[mesh_ident]\ncp_basis\n[mouseevent]\nvdv_move\n\
[newanim]\nanim_trans\nvdv_move_x\n1\n\n\
[mesh]\ndisplay.o3d\n[animparent]\ncp_basis\n\n\
[mesh]\nstatic.o3d\n[mesh_ident]\nloop_a\n[animparent]\nloop_b\n\n\
[mesh]\nother.o3d\n[mesh_ident]\nloop_b\n[animparent]\nloop_a\n";

    #[test]
    fn a_display_on_a_dragged_dashboard_stays_unclickable() {
        let m = meshes(DASHBOARD);
        assert_eq!(hot_spot_of(&m, 0), Some("vdv_move"));
        assert_eq!(hot_spot_of(&m, 1), None);
        assert_eq!(hot_spot_of(&m, 2), None, "a loop of parents ends the walk");
    }
}
