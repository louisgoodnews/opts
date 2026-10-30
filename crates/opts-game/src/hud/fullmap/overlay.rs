//! The overlay of the full-size map: root, texture node, caption, scale bar and keys.

use super::draw::FullMapImage;
use super::viewport::scale_bar_m;
use super::*;

/// The root of the overlay.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapRoot;

/// The node that shows the texture (a square that covers the screen).
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapSurface;

/// The map name and the zoom.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapCaption;

/// The bar of the scale.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapScaleBar;

/// The length of the scale bar as text.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapScaleText;

fn texts_bundle(key: &'static str, palette: &InstrumentPalette, ink: InstrumentInk) -> impl Bundle {
    (texts(key, TypeScale::Label, ink, palette), L10nKey(key))
}

/// The map name and the zoom, top left.
fn caption_panel(root: &mut ChildSpawnerCommands, palette: &InstrumentPalette) {
    root.spawn(panel(
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        RadiusLevel::Medium,
        palette,
    ))
    .with_children(|p| {
        p.spawn((
            texts("", TypeScale::BodyLarge, InstrumentInk::Text, palette),
            FullMapCaption,
            UiId::new("hud.fullmap.caption"),
        ));
    });
}

/// The scale bar, bottom left.
fn scale_panel(root: &mut ChildSpawnerCommands, palette: &InstrumentPalette) {
    root.spawn(panel(
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        RadiusLevel::Medium,
        palette,
    ))
    .with_children(|p| {
        p.spawn((
            Node {
                width: Val::Px(100.0),
                height: Val::Px(3.0),
                ..default()
            },
            BackgroundColor(palette.text),
            FullMapScaleBar,
        ));
        p.spawn((
            texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
            FullMapScaleText,
            UiId::new("hud.fullmap.scale"),
        ));
    });
}

/// The keys, bottom right.
fn hint_panel(root: &mut ChildSpawnerCommands, palette: &InstrumentPalette) {
    root.spawn(panel(
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            bottom: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        RadiusLevel::Medium,
        palette,
    ))
    .with_children(|p| {
        p.spawn((
            texts_bundle(keys::FULLMAP_HINT, palette, InstrumentInk::TextDim),
            UiId::new("hud.fullmap.hint"),
        ));
    });
}

/// Spawns the overlay.
fn spawn_overlay(commands: &mut Commands, palette: &InstrumentPalette, ground: Color) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            GlobalZIndex(1000),
            InGameEntity,
            FullMapRoot,
            UiId::new("hud.fullmap"),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ImageNode::default(),
                FullMapSurface,
                UiId::new("hud.fullmap.surface"),
            ));
            caption_panel(root, palette);
            scale_panel(root, palette);
            hint_panel(root, palette);
        });
}

/// The caption (the map name, with the reading progress while the overview is still being read)
/// and the text of the scale bar.
fn caption_texts(
    loc: &Localization,
    overview: &super::overview::OverviewRoads,
    map: &str,
    bar_m: f64,
) -> (String, String) {
    let bar_text = if bar_m >= 1000.0 {
        loc.t_args(
            keys::FULLMAP_SCALE_KM,
            &[("km".to_owned(), Arg::from((bar_m / 1000.0).round() as i32))],
        )
    } else {
        loc.t_args(
            keys::FULLMAP_SCALE_M,
            &[("metres".to_owned(), Arg::from(bar_m.round() as i32))],
        )
    };
    let name = if map.is_empty() {
        loc.t(keys::FULLMAP_TITLE)
    } else {
        map.to_owned()
    };
    // While the reader works on the other tiles, the title says how far it is.
    let title = if overview.started() && !overview.complete() {
        loc.t_args(
            keys::FULLMAP_READING,
            &[
                ("map".to_owned(), Arg::from(name.as_str())),
                ("percent".to_owned(), Arg::from(overview.percent() as i32)),
            ],
        )
    } else {
        name
    };
    (title, bar_text)
}

/// Builds the overlay when the map opens and removes it when it closes; keeps the texture node,
/// the caption and the scale bar in step with the window and the zoom.
#[allow(clippy::too_many_arguments)]
pub fn overlay(
    mut commands: Commands,
    state: Res<FullMapState>,
    image: Res<FullMapImage>,
    roots: Query<Entity, With<FullMapRoot>>,
    mut surface: Query<(&mut Node, &mut ImageNode), With<FullMapSurface>>,
    mut caption: Query<&mut Text, (With<FullMapCaption>, Without<FullMapScaleText>)>,
    mut scale_text: Query<&mut Text, (With<FullMapScaleText>, Without<FullMapCaption>)>,
    mut scale_bar: Query<&mut Node, (With<FullMapScaleBar>, Without<FullMapSurface>)>,
    session: Option<Res<InGameSessionInfo>>,
    overview: Res<super::overview::OverviewRoads>,
    loc: Res<Localization>,
    variant: Res<InstrumentVariant>,
    theme: Option<Res<MaterialTheme>>,
) {
    let existing = roots.iter().next();
    if !state.open {
        if let Some(root) = existing {
            commands.entity(root).try_despawn();
        }
        return;
    }
    let palette = *theme
        .as_ref()
        .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
            t.instrument(*variant)
        });
    if existing.is_none() {
        let ground = palette.panel;
        spawn_overlay(&mut commands, &palette, ground);
        return;
    }
    let screen = state.screen;
    // The texture covers the square around the screen; the root clips it.
    let side = screen.max_element();
    for (mut node, mut img) in &mut surface {
        let (w, h) = (Val::Px(side), Val::Px(side));
        let (left, top) = (
            Val::Px((screen.x - side) / 2.0),
            Val::Px((screen.y - side) / 2.0),
        );
        if node.width != w || node.height != h || node.left != left || node.top != top {
            node.width = w;
            node.height = h;
            node.left = left;
            node.top = top;
        }
        if let Some(h) = &image.0 {
            if img.image != *h {
                img.image = h.clone();
            }
        }
    }
    let ppm = state.viewport.px_per_m(screen);
    let bar_m = scale_bar_m(SCALE_BAR_MAX_PX / ppm);
    let map = session.as_ref().map_or("", |s| s.map_name.as_str());
    let (title, bar_text) = caption_texts(&loc, &overview, map, bar_m);
    for mut t in &mut caption {
        if t.0 != title {
            t.0.clone_from(&title);
        }
    }
    for mut t in &mut scale_text {
        if t.0 != bar_text {
            t.0.clone_from(&bar_text);
        }
    }
    for mut node in &mut scale_bar {
        let w = Val::Px((bar_m * ppm) as f32);
        if node.width != w {
            node.width = w;
        }
    }
}
