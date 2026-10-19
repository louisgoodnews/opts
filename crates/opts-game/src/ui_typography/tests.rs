//! Tests of the text preferences: the settings mapping and the font folder.

use super::*;

#[test]
fn the_size_is_read_clamped_and_defaults_to_14() {
    let mut s = GameSettings::default();
    assert_eq!(wanted(&s).base_px, 14.0, "nothing stored: 14 px");
    for (stored, want) in [
        ("20", 20.0),
        ("11", 12.0),
        ("99", 24.0),
        ("x", 14.0),
        (" 16 ", 16.0),
    ] {
        s.set_raw(SETTING_SIZE, stored);
        assert_eq!(wanted(&s).base_px, want, "stored {stored:?}");
    }
}

#[test]
fn the_family_is_read_trimmed() {
    let mut s = GameSettings::default();
    assert_eq!(wanted(&s).family, "");
    s.set_raw(SETTING_FONT, "  OpenDyslexic ");
    assert_eq!(wanted(&s).family, "OpenDyslexic");
}

#[test]
fn only_font_files_of_a_sensible_size_are_read_from_the_folder_sorted() {
    let dir = std::env::temp_dir().join(format!("opts-fonts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("b.TTF"), b"b").expect("b");
    std::fs::write(dir.join("a.otf"), b"a").expect("a");
    std::fs::write(dir.join("notes.txt"), b"x").expect("txt");
    std::fs::create_dir_all(dir.join("sub.ttf")).expect("dir named like a font");
    let files = read_font_files(&dir);
    let names: Vec<&str> = files.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(names, ["a.otf", "b.TTF"]);
    assert!(read_font_files(&dir.join("missing")).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

mod refusal {
    use super::*;
    use crate::settings_registry::{SettingsCapabilities, SettingsRegistry};

    fn app() -> App {
        crate::isolation::isolate_user_dirs();
        let dir = std::env::temp_dir().join(format!("opts-ui-font-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let mut settings = GameSettings::default();
        settings.db_path_override = Some(dir.join("settings.db"));
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_asset::<Font>()
            .insert_resource(settings)
            .insert_resource(SettingsRegistry::with_builtin())
            .init_resource::<SettingsCapabilities>()
            .add_message::<SettingChanged>()
            .init_resource::<FontCx>()
            .init_resource::<UiTypography>()
            .init_resource::<FontChoices>()
            .add_systems(PreUpdate, sync_typography)
            .add_systems(Update, (refuse_unknown_font, list_choices).chain());
        app
    }

    fn set(app: &mut App, value: &str) {
        crate::settings_registry::apply_change(
            app.world_mut(),
            SETTING_FONT,
            Some(value),
            ChangeSource::Page,
        )
        .expect("the registry accepts any text");
        for _ in 0..3 {
            app.update();
        }
    }

    #[test]
    fn a_family_the_database_does_not_hold_is_given_back_and_a_known_one_stays() {
        let mut app = app();
        // A font that is registered: the bundled DejaVu Sans.
        app.world_mut()
            .resource_mut::<Assets<Font>>()
            .add(Font::from_bytes(opts_gui::fonts::DEJAVU_SANS.to_vec()));
        for _ in 0..3 {
            app.update();
        }
        // Registering needs Bevy's loader system; do it by hand in this minimal app.
        {
            let blob = opts_gui::fonts::DEJAVU_SANS.to_vec();
            let mut cx = app.world_mut().resource_mut::<FontCx>();
            cx.collection.register_fonts(blob.into(), None);
        }
        set(&mut app, "DejaVu Sans");
        assert_eq!(app.world().resource::<UiTypography>().family, "DejaVu Sans");
        set(&mut app, "No Such Typeface");
        assert_eq!(
            app.world()
                .resource::<GameSettings>()
                .raw(SETTING_FONT)
                .as_deref(),
            Some("DejaVu Sans"),
            "the previous family is back"
        );
        assert_eq!(app.world().resource::<UiTypography>().family, "DejaVu Sans");
    }
}
