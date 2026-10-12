//! "Feature not available yet!": one component, one style, one registry.
//!
//! Decision D-UI-09 (`docs/src/roadmap/phase-9/README.md`): a feature that does not work yet stays
//! *visible* and carries the red text **"Feature not available yet!"**; it is never a silently
//! greyed button. [`NotAvailableYet`] does that for a control (disabled, tooltip, red label), and
//! every use names a [`NotYetPlace`] of the static registry [`PLACES`], so the list in
//! `docs/src/reference/not-available-yet.md` is generated, drift-checked, and a test fails when a
//! place waits for a milestone that is already complete.

use std::borrow::Cow;

use bevy::prelude::*;

use crate::interaction::state::{Disabled, DisabledReason};
use crate::l10n::L10nKey;
use crate::theme::{color::ColorRole, typography::TypeScale};
use crate::tokens::components::{BackgroundRole, ForegroundRole, TypeStyleToken};

/// The text, exactly as the decision words it.
pub const MESSAGE: &str = "Feature not available yet!"; // l10n-ok: the decision wording, also the `en` message

/// One place in the UI that waits for a feature.
#[derive(Debug, PartialEq, Eq)]
pub struct NotYetPlace {
    /// Stable id (`menu.load-last-game`).
    pub id: &'static str,
    /// Where the user meets it.
    pub place: &'static str,
    /// The milestone that delivers the feature (`M-0062`).
    pub milestone: &'static str,
    /// What is missing, in one line.
    pub missing: &'static str,
}

/// Home, *Load game*.
pub const HOME_LOAD_GAME: NotYetPlace = NotYetPlace {
    id: "home.load-game",
    place: "Home, Load game",
    milestone: "M-0062",
    missing: "There is no save system; Home offers Continue for the last drive.",
};

/// Rail, *Library*.
pub const NAV_LIBRARY: NotYetPlace = NotYetPlace {
    id: "nav.library",
    place: "Navigation rail, Library",
    milestone: "M-0062",
    missing: "The Library needs the master-detail pattern of the Plan a drive wizard and its detail panels.",
};

/// Rail, *Workshop*.
pub const NAV_WORKSHOP: NotYetPlace = NotYetPlace {
    id: "nav.workshop",
    place: "Navigation rail, Workshop",
    milestone: "M-0051",
    missing: "Editor projects, installed mods and packaging need the editor and mod system.",
};

/// Pause overlay, *Settings*.
pub const PAUSE_SETTINGS: NotYetPlace = NotYetPlace {
    id: "pause.settings",
    place: "Pause overlay, Settings",
    milestone: "M-0063",
    missing: "The settings pages build into the shell, which is hidden during a drive; they need an embeddable pane.",
};

/// Help, *Guides*.
pub const HELP_GUIDES: NotYetPlace = NotYetPlace {
    id: "help.guides",
    place: "Help, Guides",
    milestone: "M-0064",
    missing: "The short guides are written with the HUD and help overlay of the HUD milestone.",
};

/// Help, the generated list of controls.
pub const HELP_CONTROLS_GENERATED: NotYetPlace = NotYetPlace {
    id: "help.controls-generated",
    place: "Help, Controls (generated from the bindings)",
    milestone: "M-0064",
    missing: "The table is written by hand from the HUD's key hints until the in-game controls source exists.",
};

/// *Plan a drive*, step *When*, the *Live weather* card.
pub const LIVE_WEATHER: NotYetPlace = NotYetPlace {
    id: "wizard.live-weather",
    place: "Plan a drive, When, Live weather",
    milestone: "M-0062",
    missing:
        "The weather query is never called; live conditions need an explicit, explained opt-in.",
};

/// Settings, Audio: the volume settings.
pub const SETTINGS_AUDIO: NotYetPlace = NotYetPlace {
    id: "settings.audio",
    place: "Settings, Audio, volumes",
    milestone: "M-0027",
    missing: "There is no audio engine; the volume settings appear with it.",
};

/// Settings, Graphics: view distance.
pub const SETTINGS_VIEW_DISTANCE: NotYetPlace = NotYetPlace {
    id: "settings.view-distance",
    place: "Settings, Graphics, View distance",
    milestone: "M-0029",
    missing: "Tiles and objects are loaded at a fixed distance; a user setting needs the LOD and culling system.",
};

/// Settings, Gameplay: time scale of the session clock.
pub const SETTINGS_TIME_SCALE: NotYetPlace = NotYetPlace {
    id: "settings.time-scale",
    place: "Settings, Gameplay, Time scale",
    milestone: "M-0064",
    missing: "The session clock cannot be sped up or slowed down from the interface yet.",
};

/// Settings, Controls: rebinding of keys, gamepad and wheel.
pub const SETTINGS_REBIND: NotYetPlace = NotYetPlace {
    id: "settings.rebind",
    place: "Settings, Controls, Change a binding",
    milestone: "M-0035",
    missing: "Vehicle and camera keys are fixed in code; the list shows them, rebinding needs the vehicle control layer.",
};

/// Every registered place. Add new ones here; the generated list and the tests follow.
pub const PLACES: &[&NotYetPlace] = &[
    &HOME_LOAD_GAME,
    &NAV_LIBRARY,
    &NAV_WORKSHOP,
    &PAUSE_SETTINGS,
    &HELP_GUIDES,
    &HELP_CONTROLS_GENERATED,
    &LIVE_WEATHER,
    &SETTINGS_AUDIO,
    &SETTINGS_VIEW_DISTANCE,
    &SETTINGS_TIME_SCALE,
    &SETTINGS_REBIND,
];

/// What a control offers right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Works.
    Enabled,
    /// Temporarily unavailable; the reason is shown as tooltip ("Needs a saved game").
    Disabled(Cow<'static, str>),
    /// Does not exist yet; shown with the red marker.
    NotYet(&'static NotYetPlace),
}

impl Availability {
    /// The milestone of a [`Availability::NotYet`].
    pub fn milestone(&self) -> Option<&'static str> {
        match self {
            Self::NotYet(p) => Some(p.milestone),
            _ => None,
        }
    }

    /// Applies the availability to a control (replacing an earlier one).
    pub fn apply(self, entity: &mut EntityCommands) {
        entity.try_remove::<(Disabled, DisabledReason, NotAvailableYet)>();
        match self {
            Self::Enabled => {}
            Self::Disabled(reason) => {
                entity.try_insert((Disabled, DisabledReason(reason)));
            }
            Self::NotYet(place) => {
                entity.try_insert(NotAvailableYet::new(place));
            }
        }
    }
}

/// Marks a control as not available yet. Requires [`Disabled`]; adds the tooltip text and, unless
/// [`NotAvailableYet::without_label`], the red label inside the control.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[require(Disabled)]
pub struct NotAvailableYet {
    /// The registered place.
    pub place: &'static NotYetPlace,
    /// Whether to put the red label inside the control (compact controls place it themselves).
    pub label_inside: bool,
}

impl NotAvailableYet {
    /// A control with the red label inside.
    pub const fn new(place: &'static NotYetPlace) -> Self {
        Self {
            place,
            label_inside: true,
        }
    }

    /// Keeps the tooltip and the disabled look but leaves the label to the screen
    /// ([`spawn_label`]); for small inline controls.
    pub const fn without_label(mut self) -> Self {
        self.label_inside = false;
        self
    }
}

/// The red label node (one style everywhere).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NotYetLabel;

fn label_bundle() -> impl Bundle {
    (
        Text::new(MESSAGE),
        L10nKey(crate::l10n::keys::NOT_AVAILABLE),
        TextColor::default(),
        ForegroundRole(ColorRole::Error),
        TypeStyleToken(TypeScale::Label),
        // A solid surface chip behind the red text: the label keeps its 4.5:1 whatever fill the
        // disabled control has (red on the primary fill measured 4.47:1; M-0065 audit).
        BackgroundColor::default(),
        BackgroundRole(ColorRole::Surface),
        Node {
            padding: UiRect::axes(Val::Px(4.0), Val::Px(0.0)),
            ..default()
        },
        // Above the disabled scrim (z 1) of the control it sits in: the marker is never faded.
        ZIndex(2),
        NotYetLabel,
    )
}

/// Spawns a standalone red label as a child (for places that are not a control).
pub fn spawn_label(parent: &mut ChildSpawnerCommands) -> Entity {
    parent.spawn(label_bundle()).id()
}

/// Gives a newly marked control its tooltip reason and its red label.
pub fn apply_not_available_yet(
    mut commands: Commands,
    added: Query<(Entity, &NotAvailableYet, Option<&mut Node>), Added<NotAvailableYet>>,
) {
    for (entity, marker, node) in added {
        commands
            .entity(entity)
            .try_insert(DisabledReason(Cow::Owned(format!(
                "{MESSAGE} ({})",
                marker.place.milestone
            ))));
        if marker.label_inside {
            if let Some(mut node) = node {
                node.flex_direction = FlexDirection::Column;
                node.align_items = AlignItems::Center;
                node.row_gap = Val::Px(2.0);
            }
            let label = commands.spawn(label_bundle()).id();
            commands.entity(entity).add_child(label);
        }
    }
}

/// Writes the registered places to the log in debug builds.
pub fn log_places() {
    if cfg!(debug_assertions) {
        for place in PLACES {
            tracing::info!(
                id = place.id,
                milestone = place.milestone,
                "not available yet: {}",
                place.place
            );
        }
    }
}

/// Registers the systems.
pub fn build(app: &mut App) {
    app.add_systems(Startup, log_places)
        .add_systems(Update, apply_not_available_yet);
}

/// The reference page generated from [`PLACES`].
pub fn reference_markdown() -> String {
    let mut out = String::from(
        "<!-- Generated from `crates/opts-gui/src/not_yet.rs` by the test `reference_page_is_up_to_date`; \
         regenerate with `UPDATE_NOT_YET=1 cargo test -p opts-gui not_yet`. -->\n\n\
         # Features Marked \"Feature not available yet!\"\n\n\
         Decision D-UI-09 ([phase 9](../roadmap/phase-9/README.md#decisions)): a feature that does not \
         work yet stays visible and carries the red text *Feature not available yet!*. Every place is \
         registered in `opts_gui::not_yet::PLACES`; a test fails when a place waits for a milestone that is \
         already complete, so this list can only shrink by shipping the feature.\n\n\
         | Place | Id | Milestone | What is missing |\n| :--- | :--- | :--- | :--- |\n",
    );
    for p in PLACES {
        out.push_str(&format!(
            "| {} | `{}` | {} | {} |\n",
            p.place, p.id, p.milestone, p.missing
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn docs() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/src")
    }

    #[test]
    fn reference_page_is_up_to_date() {
        let path = docs().join("reference/not-available-yet.md");
        let want = reference_markdown();
        if std::env::var_os("UPDATE_NOT_YET").is_some() {
            std::fs::write(&path, &want).expect("write reference page");
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            have, want,
            "docs/src/reference/not-available-yet.md is stale: run `UPDATE_NOT_YET=1 cargo test -p opts-gui not_yet`"
        );
    }

    #[test]
    fn places_have_unique_ids_and_known_open_milestones() {
        let mut ids = std::collections::HashSet::new();
        for p in PLACES {
            assert!(ids.insert(p.id), "duplicate place id {}", p.id);
            let file = std::fs::read_dir(docs().join("roadmap"))
                .expect("roadmap dir")
                .flatten()
                .map(|d| d.path().join(format!("{}.md", p.milestone)))
                .find(|f| f.exists())
                .unwrap_or_else(|| panic!("{} names unknown milestone {}", p.id, p.milestone));
            let text = std::fs::read_to_string(file).expect("milestone page");
            let status = text
                .lines()
                .find(|l| l.contains("**Status**"))
                .unwrap_or_default();
            assert!(
                !status.contains("🟢 Complete"),
                "{} waits for {} which is marked complete: remove the marker, the feature exists",
                p.id,
                p.milestone
            );
        }
    }

    #[test]
    fn availability_applies_and_clears() {
        let mut app = crate::testing::headless_app();
        let e = app.world_mut().spawn_empty().id();
        {
            let mut commands = app.world_mut().commands();
            Availability::NotYet(&HOME_LOAD_GAME).apply(&mut commands.entity(e));
        }
        app.update();
        app.update();
        let w = app.world();
        assert!(w.get::<Disabled>(e).is_some());
        assert_eq!(
            w.get::<DisabledReason>(e).map(|r| r.0.to_string()),
            Some("Feature not available yet! (M-0062)".to_string())
        );
        {
            let mut commands = app.world_mut().commands();
            Availability::Enabled.apply(&mut commands.entity(e));
        }
        app.update();
        assert!(app.world().get::<Disabled>(e).is_none());
        assert!(app.world().get::<NotAvailableYet>(e).is_none());
    }

    #[test]
    fn the_label_is_red_text_with_the_exact_wording() {
        let mut app = crate::testing::headless_app();
        let button = {
            let mut commands = app.world_mut().commands();
            crate::components::button::ButtonBuilder::tonal("Load").spawn(&mut commands)
        };
        app.update();
        app.world_mut()
            .entity_mut(button)
            .insert(NotAvailableYet::new(&HOME_LOAD_GAME));
        app.update();
        app.update();
        let labels: Vec<String> = app
            .world_mut()
            .query_filtered::<(&Text, &ForegroundRole), With<NotYetLabel>>()
            .iter(app.world())
            .map(|(t, r)| {
                assert_eq!(r.0, ColorRole::Error);
                t.0.clone()
            })
            .collect();
        assert_eq!(labels, vec!["Feature not available yet!".to_string()]);
    }
}
