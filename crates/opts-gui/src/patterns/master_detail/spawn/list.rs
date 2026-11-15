//! Spawning of the master list column, its chips, its viewport, scroll track and thumb.

use bevy::prelude::*;

use super::container;
use super::notice::spawn_notice;
use crate::interaction::focus::{FocusRegion, Focusable, Landmark, OwnsVerticalArrows};
use crate::interaction::layer::StateLayered;
use crate::patterns::master_detail::{MasterDetailParts, MasterList, MasterScrollThumb};
use crate::theme::{color::ColorRole, shape::RadiusLevel};
use crate::tokens::components::{BackgroundRole, BorderRole, RadiusToken};
use crate::ui_id::UiId;

fn spawn_track_and_thumb(commands: &mut Commands, p: MasterDetailParts) {
    commands.entity(p.track).insert((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(2.0),
            top: Val::Px(2.0),
            bottom: Val::Px(2.0),
            width: Val::Px(8.0),
            display: Display::None,
            ..default()
        },
        Name::new("MasterScrollTrack"),
    ));
    commands.entity(p.thumb).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            height: Val::Px(24.0),
            ..default()
        },
        BackgroundRole(ColorRole::Outline),
        RadiusToken(RadiusLevel::Full),
        MasterScrollThumb(p.root),
        Name::new("MasterScrollThumb"),
    ));
    commands.entity(p.track).add_child(p.thumb);
}

fn spawn_viewport(commands: &mut Commands, p: MasterDetailParts, prefix: &str) {
    commands.entity(p.list).insert((
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            position_type: PositionType::Relative,
            overflow: Overflow::clip_y(),
            border: UiRect::all(Val::Px(1.0)),
            ..container(FlexDirection::Column)
        },
        StateLayered,
        Focusable::default(),
        OwnsVerticalArrows,
        MasterList(p.root),
        BackgroundRole(ColorRole::SurfaceContainerLow),
        BorderRole(ColorRole::OutlineVariant),
        RadiusToken(RadiusLevel::Medium),
        Name::new("MasterListViewport"),
    ));
    commands.entity(p.content).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            height: Val::Px(0.0),
            ..default()
        },
        Name::new("MasterListContent"),
    ));
    spawn_notice(commands, p, prefix);
    commands
        .entity(p.list)
        .insert(UiId::new(format!("{prefix}.list")));
    spawn_track_and_thumb(commands, p);
    commands
        .entity(p.list)
        .add_children(&[p.content, p.notice, p.track]);
}

pub(super) fn spawn_list_column(commands: &mut Commands, p: MasterDetailParts, wrap: Entity, prefix: &str) {
    commands.entity(p.list_col).insert((
        Node {
            width: Val::Percent(40.0),
            min_width: Val::Px(320.0),
            max_width: Val::Px(520.0),
            row_gap: Val::Px(8.0),
            ..container(FlexDirection::Column)
        },
        FocusRegion(Landmark::Content),
        Name::new("MasterList"),
    ));
    commands.entity(p.chips).insert((
        Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..container(FlexDirection::Row)
        },
        Name::new("MasterChips"),
    ));
    commands.entity(wrap).insert((
        Node {
            flex_grow: 1.0,
            position_type: PositionType::Relative,
            ..container(FlexDirection::Column)
        },
        Name::new("MasterListWrap"),
    ));
    spawn_viewport(commands, p, prefix);
    commands.entity(wrap).add_child(p.list);
    commands
        .entity(p.list_col)
        .add_children(&[p.search, p.chips, wrap]);
}
