//! The settings Advanced page's hardware block: what was detected and where it is stored.

use bevy::prelude::*;
use opts_gui::prelude::*;

use super::super::rows::{label, Ctx};
use super::line;

/// The detected hardware (M-0081): what OPTS knows about this machine, and where it is stored.
pub(super) fn hardware_info(c: &mut Commands, pane: Entity, ctx: &Ctx) {
    label(
        c,
        pane,
        "settings-info-hw-title",
        TypeScale::TitleMedium,
        ColorRole::OnSurface,
    );
    label(
        c,
        pane,
        "settings-info-hw-privacy",
        TypeScale::BodyMedium,
        ColorRole::OnSurfaceVariant,
    );
    let Some(profile) = ctx.hardware else {
        label(
            c,
            pane,
            "settings-info-hw-detecting",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        );
        return;
    };
    for (key, vars) in crate::hardware::view::lines(profile, ctx.loc) {
        let args: Vec<(&str, String)> = vars;
        line(c, pane, key, &args, TypeScale::BodyMedium);
    }
    let _ = match &ctx.hardware_file {
        Some(p) => line(
            c,
            pane,
            "settings-info-hw-stored",
            &[("path", p.display().to_string())],
            TypeScale::BodyMedium,
        ),
        None => label(
            c,
            pane,
            "settings-info-hw-not-stored",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ),
    };
}
