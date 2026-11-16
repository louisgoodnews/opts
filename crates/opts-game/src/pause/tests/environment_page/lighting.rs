//! The lighting-quality row of the environment page.

use super::*;

#[test]
fn the_lighting_quality_row_shows_the_mode_and_level_and_toggles_it() {
    use opts_bevy::light_quality::{GovernorState, LightMode, LightQuality};
    let mut app = drive_at_noon();
    open_page(&mut app);
    app.update();
    assert_eq!(light_row_text(&mut app), "Automatic (level 0: full)");
    app.world_mut().resource_mut::<GovernorState>().level = 2;
    app.update();
    app.update();
    assert_eq!(
        light_row_text(&mut app),
        "Automatic (level 2: headlights only)"
    );
    click(&mut app, "pause.environment.light-quality");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(light_row_text(&mut app), "Custom");
    let q = *app.world().resource::<LightQuality>();
    assert_eq!((q.mode, q.level), (LightMode::Custom, 2));
    click(&mut app, "pause.environment.light-quality");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(light_row_text(&mut app), "Automatic (level 0: full)");
    assert!(paused(&app), "the page still pauses the drive");
}
