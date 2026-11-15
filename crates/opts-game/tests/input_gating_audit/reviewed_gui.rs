//! Reviewed input-reading files in `opts-gui` (paths relative to `crates/`).

/// Files whose systems read input and were reviewed.
pub(super) const REVIEWED: &[&str] = &[
    "opts-gui/src/actions.rs",
    "opts-gui/src/components/button.rs",
    "opts-gui/src/components/checkbox.rs",
    "opts-gui/src/components/combobox.rs",
    "opts-gui/src/components/modal.rs",
    "opts-gui/src/components/notebook.rs",
    "opts-gui/src/components/radio.rs",
    "opts-gui/src/components/switch.rs",
    "opts-gui/src/components/text_area/input.rs",
    "opts-gui/src/components/text_field.rs",
    "opts-gui/src/components/toast.rs",
    "opts-gui/src/components/toplevel.rs",
    "opts-gui/src/components/treeview.rs",
    "opts-gui/src/interaction/activation.rs",
    "opts-gui/src/interaction/focus.rs",
    "opts-gui/src/interaction/layer.rs",
    "opts-gui/src/interaction/layer/tests.rs",
    "opts-gui/src/interaction/mod.rs",
    "opts-gui/src/interaction/state.rs",
    // Patterns: keyboard only while their list has focus (focus is trapped by modals).
    "opts-gui/src/patterns/master_detail_input.rs",
    // Split out of `master_detail_input.rs`, same rule.
    "opts-gui/src/patterns/master_detail_input/keys.rs",
];
