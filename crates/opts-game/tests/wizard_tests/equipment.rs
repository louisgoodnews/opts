//! The equipment dialog and saved configurations.

use super::flow::choose_in_what;
use super::*;

/// Opens the equipment dialog on a vehicle whose repaints carry a switch, a choice and a number.
fn open_equipment_dialog(app: &mut App) {
    use bevy::ecs::system::RunSystemOnce;
    use opts_game::new_game_screen::livery::LiveryState;
    use opts_game::new_game_screen::options::open_options;
    use opts_game::new_game_screen::repaints::VehicleRepaints;
    use opts_gui::l10n::Localization;

    let mut reps = VehicleRepaints {
        default_name: "Red".into(),
        ..VehicleRepaints::default()
    };
    reps.setvars.insert(
        "Red".into(),
        vec![
            ("vis_x".into(), "1".into()),
            ("display_ext".into(), "2".into()),
            ("vehicle_id".into(), "3964".into()),
        ],
    );
    reps.setvars
        .insert("Blue".into(), vec![("display_ext".into(), "5".into())]);
    app.world_mut().resource_mut::<LiveryState>().repaints = Some(reps);
    app.world_mut()
        .run_system_once(
            |mut commands: Commands,
             loc: Res<Localization>,
             catalogue: Res<opts_game::new_game_screen::GameCatalogue>,
             setup: Res<NewGameSetup>,
             liveries: Res<LiveryState>| {
                open_options(&mut commands, &loc, &catalogue, &setup, &liveries);
            },
        )
        .expect("open the dialog");
    for _ in 0..3 {
        app.update();
    }
}

/// Writes through the dialog's controls: a choice, a number and a switch.
fn exercise_option_controls(app: &mut App, (switch, combo, field): (Entity, Entity, Entity)) {
    use opts_gui::interaction::events::{ComboboxSelected, SwitchToggled, TextChanged};
    // Choosing the second value (5) writes it; choosing the default again removes the override.
    app.world_mut().write_message(ComboboxSelected {
        entity: combo,
        selected_index: 1,
        selected_value: "5".into(),
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<NewGameSetup>()
            .custom_setvars
            .get("display_ext"),
        Some(&5)
    );
    app.world_mut().write_message(ComboboxSelected {
        entity: combo,
        selected_index: 0,
        selected_value: "2".into(),
    });
    app.update();
    assert!(!app
        .world()
        .resource::<NewGameSetup>()
        .custom_setvars
        .contains_key("display_ext"));

    // A number: valid text is written, text that is no number or out of range is ignored.
    for (text, want) in [
        ("100", Some(100)),
        ("abc", Some(100)),
        ("-5", Some(100)),
        ("3964", None),
    ] {
        app.world_mut().write_message(TextChanged {
            entity: field,
            text: text.into(),
        });
        app.update();
        assert_eq!(
            app.world()
                .resource::<NewGameSetup>()
                .custom_setvars
                .get("vehicle_id")
                .copied(),
            want,
            "after typing {text:?}"
        );
    }

    // The switch writes 0 against its default 1.
    app.world_mut().write_message(SwitchToggled {
        entity: switch,
        is_on: false,
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<NewGameSetup>()
            .custom_setvars
            .get("vis_x"),
        Some(&0)
    );
}

#[test]
fn the_equipment_dialog_offers_a_control_that_fits_each_option_and_writes_the_choices() {
    use opts_game::new_game_screen::options::{OptionChoice, OptionNumber, OptionSwitch};

    let mut app = wizard("options-dialog");
    open_equipment_dialog(&mut app);

    let switch = find_entity(app.world_mut(), "wizard.what.option.vis-x").expect("a switch");
    assert!(app.world().get::<OptionSwitch>(switch).is_some());
    let combo = find_entity(app.world_mut(), "wizard.what.option.display-ext").expect("a choice");
    let choice = app
        .world()
        .get::<OptionChoice>(combo)
        .expect("a choice box")
        .clone();
    assert_eq!(
        choice.values,
        vec![2, 5],
        "what the repaints give; 0 only when the repaint leaves it unset"
    );
    assert_eq!(choice.default, 2);
    let field = find_entity(app.world_mut(), "wizard.what.option.vehicle-id").expect("a number");
    assert!(app.world().get::<OptionNumber>(field).is_some());

    exercise_option_controls(&mut app, (switch, combo, field));
    cleanup("options-dialog");
}

mod library;
