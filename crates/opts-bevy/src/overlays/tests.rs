//! Unit tests of `overlays`: the level text, the kind of each weather class and the filter.

use super::*;
use crate::model::ModelPart;

fn part(variable: Option<&str>) -> ModelPart {
    ModelPart {
        mesh: Handle::default(),
        material: Handle::default(),
        vertices: 0,
        light_variable: None,
        alpha_variable: variable.map(str::to_owned),
        image: None,
        free_texture: None,
        pivot: None,
    }
}

#[test]
fn level_text_round_trips_and_unknown_text_is_none() {
    for level in OverlayLevel::ALL {
        assert_eq!(OverlayLevel::parse(level.name()), Some(level));
    }
    assert_eq!(OverlayLevel::parse(" FULL "), Some(OverlayLevel::Full));
    assert_eq!(OverlayLevel::parse("medium"), None);
    assert_eq!(OverlayLevel::default(), OverlayLevel::Full);
}

#[test]
fn levels_keep_the_kinds_the_setting_names() {
    use OverlayKind::{Body, Window};
    assert!(!OverlayLevel::Off.keeps(Window) && !OverlayLevel::Off.keeps(Body));
    assert!(OverlayLevel::Simple.keeps(Window) && !OverlayLevel::Simple.keeps(Body));
    assert!(OverlayLevel::Full.keeps(Window) && OverlayLevel::Full.keeps(Body));
}

#[test]
fn weather_variables_map_to_window_or_body() {
    assert_eq!(
        OverlayKind::of_variable("Rain_Window_Norm_Wetness"),
        Some(OverlayKind::Window)
    );
    assert_eq!(
        OverlayKind::of_variable("Beschlag_Front"),
        Some(OverlayKind::Window)
    );
    assert_eq!(
        OverlayKind::of_variable("Dirt_Norm"),
        Some(OverlayKind::Body)
    );
    assert_eq!(
        OverlayKind::of_variable("Precip_Mirror"),
        Some(OverlayKind::Body)
    );
    assert_eq!(OverlayKind::of_variable("Envir_Brightness"), None);
    assert_eq!(OverlayKind::of_variable("szyby"), None);
}

#[test]
fn the_filter_drops_only_weather_overlays_the_level_does_not_keep() {
    let dirt = part(Some("Dirt_Norm"));
    let glass = part(Some("Envir_Brightness"));
    let plain = part(None);
    assert!(!part_kept(OverlayLevel::Off, &dirt));
    assert!(!part_kept(OverlayLevel::Simple, &dirt));
    assert!(part_kept(OverlayLevel::Full, &dirt));
    for level in OverlayLevel::ALL {
        assert!(part_kept(level, &glass), "glass is no weather overlay");
        assert!(part_kept(level, &plain), "plain part");
    }
    assert!(part_kept(
        OverlayLevel::Simple,
        &part(Some("Beschlag_Front"))
    ));
}

#[test]
fn off_removes_every_overlay_entity_and_simple_keeps_the_windows() {
    fn count_after(level: OverlayLevel) -> usize {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.world_mut().spawn(OverlayKind::Window);
        app.world_mut().spawn(OverlayKind::Body);
        app.add_systems(
            Update,
            move |mut commands: Commands, q: Query<(Entity, &OverlayKind)>| {
                despawn_disallowed(&mut commands, level, &q);
            },
        );
        app.update();
        app.world_mut()
            .query::<&OverlayKind>()
            .iter(app.world())
            .count()
    }
    assert_eq!(count_after(OverlayLevel::Off), 0);
    assert_eq!(count_after(OverlayLevel::Simple), 1);
    assert_eq!(count_after(OverlayLevel::Full), 2);
}
