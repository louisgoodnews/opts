//! Spawning the loading screen widgets: manifest tiles, progress bar and controls card.

use super::*;

mod controls;
mod progress;
use controls::controls_card;
use progress::progress_block;

fn tile_label(t: Tile) -> (&'static str, ColorRole) {
    match t {
        Tile::Map => ("load-tile-map", ColorRole::Primary),
        Tile::Vehicle => ("load-tile-vehicle", ColorRole::Tertiary),
        Tile::When => ("load-tile-when", ColorRole::Secondary),
    }
}

fn manifest_tile(commands: &mut Commands, tile: Tile) -> Entity {
    let (label, role) = tile_label(tile);
    let frame = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..column(4.0)
            },
            BackgroundRole(ColorRole::SurfaceContainerLow),
            BorderRole(ColorRole::OutlineVariant),
            RadiusToken(RadiusLevel::Medium),
            Name::new("ManifestTile"),
        ))
        .id();
    let head = commands.spawn(keyed(label, TypeScale::Label, role)).id();
    let title = commands
        .spawn((
            plain(String::new(), TypeScale::TitleMedium, ColorRole::OnSurface),
            ManifestText(tile, 0),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ))
        .id();
    let sub = commands
        .spawn((
            plain(
                String::new(),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ),
            ManifestText(tile, 1),
            TextLayout::new(Justify::Left, LineBreak::NoWrap),
        ))
        .id();
    commands.entity(frame).add_children(&[head, title, sub]);
    frame
}

/// Spawns the loading screen (inside the shell's content area when there is one).
pub fn spawn_loading_screen_ui(commands: &mut Commands, shell: &ShellContent) -> Entity {
    let root = spawn_screen_root(commands, shell, "Loading");
    commands.entity(root).insert((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..column(0.0)
        },
        BackgroundRole(ColorRole::Surface),
        GlobalZIndex(100),
        InGameEntity,
        LoadingScreenRoot,
    ));
    let inner = commands
        .spawn(Node {
            width: Val::Percent(92.0),
            max_width: Val::Px(960.0),
            ..column(16.0)
        })
        .id();
    let title = commands
        .spawn(keyed(
            "load-title",
            TypeScale::Headline,
            ColorRole::OnSurface,
        ))
        .id();
    let manifest = commands.spawn(row(12.0)).id();
    let tiles: Vec<Entity> = [Tile::Map, Tile::Vehicle, Tile::When]
        .into_iter()
        .map(|t| manifest_tile(commands, t))
        .collect();
    commands.entity(manifest).add_children(&tiles);
    let progress = progress_block(commands);
    let outcome = commands
        .spawn((column(8.0), OutcomeBox, Name::new("Outcome")))
        .id();
    let controls = controls_card(commands);
    let footer = commands.spawn(row(12.0)).id();
    let cancel = ButtonBuilder::outlined("load-cancel")
        .with_key("load-cancel")
        .spawn(commands);
    commands
        .entity(cancel)
        .insert((CancelLoadingBtn, UiId::new("loading.cancel")));
    commands.entity(footer).add_child(cancel);
    commands
        .entity(inner)
        .add_children(&[title, manifest, progress, outcome, controls, footer]);
    commands.entity(root).add_child(inner);
    root
}
