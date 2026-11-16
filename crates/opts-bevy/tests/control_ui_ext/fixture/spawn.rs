//! The fixture's controls: a scroll view with buttons, a switch, combo boxes and a text field, and a list.

use super::super::*;

/// A windowed list of 300 entries beside the other fixtures (the wizard's map list is one).
pub(super) fn spawn_list(app: &mut App) -> Entity {
    use opts_gui::l10n::{wizard_keys::md, Localization};
    use opts_gui::patterns::list_model::ListEntry;
    use opts_gui::patterns::master_detail::{
        spawn_master_detail, MasterDetail, MasterDetailConfig,
    };
    let loc = app.world().resource::<Localization>().clone();
    let entries: Vec<ListEntry> = (0..300)
        .map(|i| ListEntry::new(format!("id{i}"), format!("Entry {i:05}")))
        .collect();
    let mut commands = app.world_mut().commands();
    let parts = spawn_master_detail(
        &mut commands,
        &loc,
        MasterDetailConfig {
            prefix: "maps",
            empty_key: md::NO_RESULTS,
            search_key: md::SEARCH,
        },
        MasterDetail::new(entries, None),
    );
    let frame = commands
        .spawn(Node {
            width: Val::Px(900.0),
            height: Val::Px(500.0),
            ..default()
        })
        .id();
    commands.entity(frame).add_child(parts.root);
    parts.root
}

/// A scroll view of 30 buttons (the first counts its clicks), a switch, two combo boxes and a
/// text field.
pub(super) fn spawn_controls(app: &mut App, c: &Arc<AtomicUsize>) {
    let mut commands = app.world_mut().commands();
    // A 300 x 200 scroll view with 30 buttons of about 40 px.
    let view = commands
        .spawn((
            Node {
                width: Val::Px(300.0),
                height: Val::Px(200.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            UiId::new("list.view"),
        ))
        .id();
    let rows: Vec<Entity> = (0..30)
        .map(|i| {
            let mut b = ButtonBuilder::filled(format!("Row {i}"));
            if i == 0 {
                let c = c.clone();
                b = b.on_click(move |_, _| {
                    c.fetch_add(1, Ordering::SeqCst);
                });
            }
            let e = b.spawn(&mut commands);
            commands.entity(e).insert(UiId::new(format!("row.{i}")));
            e
        })
        .collect();
    commands.entity(view).add_children(&rows);
    let sw = SwitchBuilder::standalone().spawn(&mut commands);
    commands.entity(sw).insert(UiId::new("opt.switch"));
    let combo = ComboboxBuilder::new("Weather")
        .with_options(vec!["Sunny".into(), "Rain".into(), "Fog".into()])
        .with_selected(0)
        .spawn(&mut commands);
    let _ = combo;
    let long = ComboboxBuilder::new("Start")
        .with_options((0..12).map(|i| format!("Start point {i}")).collect())
        .spawn(&mut commands);
    commands.entity(long).insert(UiId::new("opt.start"));
    let field = TextFieldBuilder::outlined("Search").spawn(&mut commands);
    commands.entity(field).insert(UiId::new("opt.search"));
}
