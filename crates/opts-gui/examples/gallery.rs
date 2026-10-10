//! Comprehensive gallery demonstrating all Material Design 3 widgets in OPTS GUI.

use bevy::prelude::*;
use opts_gui::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS Material Design 3 Widget Gallery".into(),
                resolution: (1280.0_f32, 800.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OptsGuiPlugin)
        .add_systems(Startup, setup_gallery)
        .add_systems(
            Update,
            (
                theme_hotkey_toggle_system,
                gallery_interaction_listener,
                spawn_dialog_system,
                spawn_toast_system,
            ),
        )
        .run();
}

#[derive(Component)]
struct SpawnDialogButton;

#[derive(Component)]
struct SpawnToastButton;

#[derive(Component)]
struct StatusLabel;

fn setup_gallery(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Root screen layout
    let root = commands
        .spawn((
            (Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..default()
            }),
            BackgroundRole(ColorRole::Surface),
        ))
        .id();

    // 1. App Top Bar
    commands.entity(root).with_children(|parent| {
        parent
            .spawn((
                (Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(56.0),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(24.0), Val::Px(8.0)),
                    border: UiRect {
                        bottom: Val::Px(1.0),
                        ..default()
                    },
                    ..default()
                }),
                BackgroundRole(ColorRole::SurfaceContainerLow),
                BorderRole(ColorRole::OutlineVariant),
            ))
            .with_children(|bar| {
                bar.spawn(((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                }),))
                    .with_children(|title_box| {
                        LabelBuilder::new("OPTS Design System Gallery")
                            .with_style(TypeScale::TitleLarge)
                            .with_color(ColorRole::Primary)
                            .spawn_child(title_box);

                        LabelBuilder::new("— Material Design 3")
                            .with_style(TypeScale::LabelLarge)
                            .with_color(ColorRole::OnSurfaceVariant)
                            .spawn_child(title_box);
                    });

                bar.spawn(((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(16.0),
                    ..default()
                }),))
                    .with_children(|controls| {
                        LabelBuilder::new("F2: Toggle Theme | Tab: Focus")
                            .with_style(TypeScale::LabelMedium)
                            .with_color(ColorRole::Outline)
                            .spawn_child(controls);

                        ButtonBuilder::tonal("Toggle Theme")
                            .on_click(|commands, _entity| {
                                commands.add(|world: &mut World| {
                                    let is_dark = {
                                        let theme = world.resource::<MaterialTheme>();
                                        theme.scheme.is_dark
                                    };
                                    world.write_message(ThemeChanged { is_dark: !is_dark });
                                });
                            })
                            .spawn_child(controls);
                    });
            });
    });

    // 2. Main Content Notebook
    commands.entity(root).with_children(|parent| {
        let notebook = NotebookBuilder::new()
            .with_tabs(vec![
                "Inputs & Controls".into(),
                "Transit & Timetable".into(),
                "Overlays & Cards".into(),
            ])
            .with_active(0)
            .spawn_child(parent);

        // Find the panels and populate them
        populate_gallery_panels(parent, notebook);
    });

    // 3. Status Bar at Bottom
    commands.entity(root).with_children(|parent| {
        parent
            .spawn((
                (Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(32.0),
                    padding: UiRect::axes(Val::Px(24.0), Val::Px(6.0)),
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    border: UiRect {
                        top: Val::Px(1.0),
                        ..default()
                    },
                    ..default()
                }),
                BackgroundRole(ColorRole::SurfaceContainerLowest),
                BorderRole(ColorRole::OutlineVariant),
            ))
            .with_children(|status_bar| {
                status_bar.spawn((
                    (
                        Text::new("Ready. Interact with any widget to observe reactive events."),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ),
                    StatusLabel,
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                    TypeStyleToken(TypeScale::LabelMedium),
                ));
            });
    });
}

fn populate_gallery_panels(parent: &mut ChildSpawnerCommands, _notebook: Entity) {
    // Note: Notebook panels are automatically spawned inside NotebookBuilder.
    // Here we can spawn our gallery section containers inside a row layout for demonstration.
    parent
        .spawn(
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                column_gap: Val::Px(24.0),
                overflow: Overflow::clip_y(),
                ..default()
            }),
        )
        .with_children(|content| {
            // Column 1: Buttons & Inputs
            build_inputs_column(content);

            // Column 2: Selection & Toggles
            build_selection_column(content);

            // Column 3: Transit Widgets & Containers
            build_transit_column(content);
        });
}

fn build_inputs_column(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: Val::Px(16.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            }),
            BackgroundRole(ColorRole::SurfaceContainerLow),
            RadiusToken(RadiusLevel::Medium),
        ))
        .with_children(|col| {
            LabelBuilder::new("Buttons & Text Fields")
                .with_style(TypeScale::TitleMedium)
                .with_color(ColorRole::Primary)
                .spawn_child(col);

            DividerBuilder::horizontal().spawn_child(col);

            ButtonBuilder::filled("Filled Button").spawn_child(col);
            ButtonBuilder::outlined("Outlined Button").spawn_child(col);
            ButtonBuilder::text("Text Button").spawn_child(col);

            TextFieldBuilder::outlined("Bus Line")
                .with_placeholder("e.g. M29")
                .with_text("100")
                .spawn_child(col);

            TextFieldBuilder::filled("Driver Name")
                .with_placeholder("Enter name")
                .spawn_child(col);
        });
}

fn build_selection_column(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: Val::Px(16.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            }),
            BackgroundRole(ColorRole::SurfaceContainerLow),
            RadiusToken(RadiusLevel::Medium),
        ))
        .with_children(|col| {
            LabelBuilder::new("Selection & Menus")
                .with_style(TypeScale::TitleMedium)
                .with_color(ColorRole::Primary)
                .spawn_child(col);

            DividerBuilder::horizontal().spawn_child(col);

            CheckboxBuilder::new("Automated Signal Priority")
                .checked(true)
                .spawn_child(col);

            SwitchBuilder::new("Dynamic Headway Control")
                .with_value(true)
                .spawn_child(col);

            RadioBuilder::new("Standard Service")
                .with_group(1)
                .selected(true)
                .spawn_child(col);

            RadioBuilder::new("Express Service")
                .with_group(1)
                .spawn_child(col);

            ComboboxBuilder::new("Depot")
                .with_options(vec![
                    "Cicerostraße Depot".into(),
                    "Britz Depot".into(),
                    "Müllerstraße Depot".into(),
                ])
                .with_selected(0)
                .spawn_child(col);
        });
}

fn build_transit_column(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                row_gap: Val::Px(16.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            }),
            BackgroundRole(ColorRole::SurfaceContainerLow),
            RadiusToken(RadiusLevel::Medium),
        ))
        .with_children(|col| {
            LabelBuilder::new("Transit & Overlays")
                .with_style(TypeScale::TitleMedium)
                .with_color(ColorRole::Primary)
                .spawn_child(col);

            DividerBuilder::horizontal().spawn_child(col);

            DateSelectBuilder::new(2026, 10, 8).spawn_child(col);
            DateTimeSelectBuilder::new(2026, 10, 8, 14, 30).spawn_child(col);

            // Action triggers
            col.spawn((
                (
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ),
                SpawnDialogButton,
                BackgroundRole(ColorRole::SecondaryContainer),
                RadiusToken(RadiusLevel::Full),
            ))
            .with_children(|btn| {
                LabelBuilder::new("Open Modal Dialog")
                    .with_style(TypeScale::LabelLarge)
                    .with_color(ColorRole::OnSecondaryContainer)
                    .spawn_child(btn);
            });

            col.spawn((
                (
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ),
                SpawnToastButton,
                BackgroundRole(ColorRole::TertiaryContainer),
                RadiusToken(RadiusLevel::Full),
            ))
            .with_children(|btn| {
                LabelBuilder::new("Show Toast Alert")
                    .with_style(TypeScale::LabelLarge)
                    .with_color(ColorRole::OnTertiaryContainer)
                    .spawn_child(btn);
            });

            // Treeview demo
            let root_node = TreeNode::new("bvg", "BVG Network")
                .with_child(
                    TreeNode::new("bus", "Bus Network")
                        .with_child(TreeNode::new("m29", "Line M29 (Hermannplatz ↔ Roseneck)"))
                        .with_child(TreeNode::new("100", "Line 100 (Alexanderplatz ↔ Zoo)")),
                )
                .with_child(TreeNode::new("subway", "U-Bahn Network"));

            TreeviewBuilder::new().with_node(root_node).spawn_child(col);
        });
}

fn theme_hotkey_toggle_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    theme: Res<MaterialTheme>,
    mut events: MessageWriter<ThemeChanged>,
) {
    if keyboard.just_pressed(KeyCode::F2) {
        events.write(ThemeChanged {
            is_dark: !theme.scheme.is_dark,
        });
    }
}

fn spawn_dialog_system(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<SpawnDialogButton>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            ToplevelBuilder::new("Vehicle Dispatch Parameters")
                .with_size(440.0, 260.0)
                .modal(true)
                .spawn(&mut commands);
        }
    }
}

fn spawn_toast_system(
    mut commands: Commands,
    query: Query<&Interaction, (Changed<Interaction>, With<SpawnToastButton>)>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed {
            ToastBuilder::new("Timetable schedule synchronized successfully")
                .with_duration(3.5)
                .with_action("OK")
                .spawn(&mut commands);
        }
    }
}

fn gallery_interaction_listener(
    mut click_events: MessageReader<ButtonClicked>,
    mut check_events: MessageReader<CheckboxToggled>,
    mut switch_events: MessageReader<SwitchToggled>,
    mut radio_events: MessageReader<RadioSelected>,
    mut combo_events: MessageReader<ComboboxSelected>,
    mut tree_select_events: MessageReader<TreeNodeSelected>,
    mut status_query: Query<&mut Text, With<StatusLabel>>,
) {
    for mut text in &mut status_query {
        for event in click_events.read() {
            text.0 = format!("Event: ButtonClicked({:?})", event.0);
        }
        for event in check_events.read() {
            text.0 = format!("Event: CheckboxToggled({:?})", event.state);
        }
        for event in switch_events.read() {
            text.0 = format!("Event: SwitchToggled(is_on={})", event.is_on);
        }
        for event in radio_events.read() {
            text.0 = format!("Event: RadioSelected({:?})", event.entity);
        }
        for event in combo_events.read() {
            text.0 = format!("Event: ComboboxSelected('{}')", event.selected_value);
        }
        for event in tree_select_events.read() {
            text.0 = format!("Event: TreeNodeSelected('{}')", event.node_id);
        }
    }
}
