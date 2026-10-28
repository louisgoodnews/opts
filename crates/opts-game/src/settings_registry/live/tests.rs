//! Unit tests for `super` (`live.rs`).

mod view_distance_tests {
    use super::super::*;

    fn settings(value: Option<&str>) -> GameSettings {
        let mut s = GameSettings::default();
        if let Some(v) = value {
            s.set_raw("graphics.view_distance", v);
        }
        s
    }

    #[test]
    fn steps_are_150_metres_and_the_largest_is_unlimited() {
        assert_eq!(draw_distance_of(&settings(None)), Some(600.0));
        assert_eq!(draw_distance_of(&settings(Some("1"))), Some(150.0));
        assert_eq!(draw_distance_of(&settings(Some("7"))), Some(1050.0));
        assert_eq!(draw_distance_of(&settings(Some("8"))), None);
        assert_eq!(draw_distance_of(&settings(Some("0"))), Some(150.0));
        assert_eq!(draw_distance_of(&settings(Some("99"))), None);
        assert_eq!(draw_distance_of(&settings(Some("far"))), Some(600.0));
    }

    #[test]
    fn the_distance_settings_reach_the_shadows_the_lamps_and_the_lod_bias() {
        let mut app = App::new();
        app.insert_resource(settings(None))
            .init_resource::<opts_bevy::SceneryLodBias>()
            .init_resource::<opts_bevy::scenery_runtime::MapLightSettings>()
            .add_systems(Update, sync_detail_distances);
        app.world_mut().spawn(DirectionalLight::default());
        app.update();
        // The defaults: 300 m of shadows, 350 m of lamps, detail kept twice as far.
        let shadow = app
            .world_mut()
            .query::<&bevy::light::CascadeShadowConfig>()
            .single(app.world())
            .unwrap()
            .bounds
            .last()
            .copied()
            .unwrap();
        assert!((shadow - 300.0).abs() < 0.5, "{shadow}");
        assert_eq!(
            app.world()
                .resource::<opts_bevy::scenery_runtime::MapLightSettings>()
                .radius_m,
            350.0
        );
        assert_eq!(app.world().resource::<opts_bevy::SceneryLodBias>().0, 2.0);
    }

    #[test]
    fn the_setting_drives_the_scenery_draw_distance() {
        let mut app = App::new();
        app.insert_resource(settings(Some("2")))
            .init_resource::<opts_bevy::SceneryDrawDistance>()
            .add_systems(Update, sync_view_distance);
        app.update();
        assert_eq!(
            app.world()
                .resource::<opts_bevy::SceneryDrawDistance>()
                .metres,
            Some(300.0)
        );
        app.world_mut()
            .resource_mut::<GameSettings>()
            .set_raw("graphics.view_distance", "8");
        app.update();
        assert_eq!(
            app.world()
                .resource::<opts_bevy::SceneryDrawDistance>()
                .metres,
            None
        );
    }
}

mod audio_tests {
    use super::super::*;

    #[test]
    fn the_volume_settings_reach_the_engine() {
        let mut app = App::new();
        let mut settings = GameSettings::default();
        settings.set_raw("audio.master", "50");
        settings.set_raw("audio.engine", "25");
        settings.set_raw("audio.ambience", "not a number");
        app.insert_resource(settings)
            .init_resource::<opts_bevy::AudioSettings>()
            .add_systems(Update, sync_audio);
        app.update();
        let a = app.world().resource::<opts_bevy::AudioSettings>().clone();
        assert_eq!(
            (a.master, a.vehicles, a.ui, a.effects),
            (50.0, 25.0, 60.0, 80.0)
        );
        assert_eq!(a.ambient, 60.0, "an unreadable value is the default");
        assert!(a.enabled);
        app.world_mut()
            .resource_mut::<GameSettings>()
            .set_raw("audio.master", "0");
        app.update();
        assert!(!app.world().resource::<opts_bevy::AudioSettings>().enabled);
    }
}
