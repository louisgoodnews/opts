//! UI scale (75-200 %), the layout class of the window and the interim minimum text size.
//!
//! * [`UiScalePercent`] is the setting (`ui_scale`); [`apply_ui_scale`] writes it into Bevy's
//!   [`UiScale`], which multiplies every logical size including font sizes.
//! * [`LayoutClass`] is recomputed from the window width **in layout pixels** (window width
//!   divided by the UI scale) whenever either changes.
//! * [`MinTextSize`] clamps every text below 12 logical px (times UI scale, because `UiScale`
//!   multiplies afterwards). It is an **interim** guard: screens still contain literal sizes of
//!   10 and 11 px; the clamp disappears when they have migrated to the type scale (M-0062 and
//!   later).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::spacing::LayoutClass;
use super::typography::MIN_TEXT_PX;

/// Smallest UI scale in percent.
pub const UI_SCALE_MIN: u32 = 75;
/// Largest UI scale in percent.
pub const UI_SCALE_MAX: u32 = 200;
/// Step of the UI scale control in percent.
pub const UI_SCALE_STEP: u32 = 25;

/// The UI scale setting in percent (`75..=200`, default 100).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiScalePercent(pub u32);

impl UiScalePercent {
    /// Clamps any value into the allowed range.
    pub fn clamped(percent: u32) -> Self {
        Self(percent.clamp(UI_SCALE_MIN, UI_SCALE_MAX))
    }

    /// The factor for Bevy's [`UiScale`].
    pub fn factor(self) -> f32 {
        self.0.clamp(UI_SCALE_MIN, UI_SCALE_MAX) as f32 / 100.0
    }

    /// The next larger preset step, saturating at the maximum.
    pub fn increased(self) -> Self {
        Self::clamped(self.0 + UI_SCALE_STEP)
    }

    /// The next smaller preset step, saturating at the minimum.
    pub fn decreased(self) -> Self {
        Self::clamped(self.0.saturating_sub(UI_SCALE_STEP))
    }
}

impl Default for UiScalePercent {
    fn default() -> Self {
        Self(100)
    }
}

/// Smallest text size in logical pixels before UI scale.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct MinTextSize(pub f32);

impl Default for MinTextSize {
    fn default() -> Self {
        Self(MIN_TEXT_PX)
    }
}

/// Writes the [`UiScalePercent`] setting into Bevy's [`UiScale`] when it changes.
pub fn apply_ui_scale(setting: Res<UiScalePercent>, mut ui_scale: ResMut<UiScale>) {
    let factor = setting.factor();
    if setting.is_changed() && (ui_scale.0 - factor).abs() > f32::EPSILON {
        ui_scale.0 = factor;
    }
}

/// Keeps [`LayoutClass`] in step with the primary window width and the UI scale.
pub fn update_layout_class(
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut class: ResMut<LayoutClass>,
) {
    let Ok(window) = windows.single() else { return };
    let layout_width = window.width() / ui_scale.0.max(0.01);
    let new = LayoutClass::from_width(layout_width);
    if *class != new {
        *class = new;
    }
}

/// Raises every pixel font size below [`MinTextSize`] to it (interim guard, see the module docs).
pub fn clamp_text_size(min: Res<MinTextSize>, mut query: Query<&mut TextFont, Changed<TextFont>>) {
    for mut font in &mut query {
        if let FontSize::Px(px) = font.font_size {
            if px < min.0 {
                font.font_size = FontSize::Px(min.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_is_clamped_and_converted() {
        assert_eq!(UiScalePercent::clamped(10).0, 75);
        assert_eq!(UiScalePercent::clamped(500).0, 200);
        assert_eq!(UiScalePercent(150).factor(), 1.5);
        assert_eq!(UiScalePercent(100).increased().0, 125);
        assert_eq!(UiScalePercent(75).decreased().0, 75);
        assert_eq!(UiScalePercent(200).increased().0, 200);
    }

    #[test]
    fn small_text_is_raised_to_the_floor_and_larger_text_is_untouched() {
        let mut app = App::new();
        app.insert_resource(MinTextSize::default())
            .add_systems(Update, clamp_text_size);
        let small = app.world_mut().spawn(TextFont::from_font_size(10.0)).id();
        let big = app.world_mut().spawn(TextFont::from_font_size(20.0)).id();
        app.update();
        let px = |app: &App, e| match app.world().get::<TextFont>(e).map(|f| f.font_size) {
            Some(FontSize::Px(p)) => p,
            _ => -1.0,
        };
        assert_eq!(px(&app, small), 12.0);
        assert_eq!(px(&app, big), 20.0);
    }

    fn layout_class_of(width: f32, scale: f32) -> LayoutClass {
        let mut app = App::new();
        let mut window = Window::default();
        window.resolution.set(width, 700.0);
        app.insert_resource(UiScale(scale))
            .init_resource::<LayoutClass>()
            .add_systems(Update, update_layout_class);
        app.world_mut().spawn((window, PrimaryWindow));
        app.update();
        *app.world().resource::<LayoutClass>()
    }

    #[test]
    fn the_layout_class_follows_the_window_width_in_layout_pixels() {
        assert_eq!(layout_class_of(800.0, 1.0), LayoutClass::Compact);
        assert_eq!(layout_class_of(1024.0, 1.0), LayoutClass::Medium);
        assert_eq!(layout_class_of(1920.0, 1.0), LayoutClass::Expanded);
        // 1920 physical-ish pixels at 200 % scale leave 960 layout pixels: medium
        assert_eq!(layout_class_of(1920.0, 2.0), LayoutClass::Medium);
        // 1024 at 125 % leave 819 layout pixels: compact
        assert_eq!(layout_class_of(1024.0, 1.25), LayoutClass::Compact);
    }

    #[test]
    fn ui_scale_follows_the_setting() {
        let mut app = App::new();
        app.insert_resource(UiScale::default())
            .insert_resource(UiScalePercent(150))
            .add_systems(Update, apply_ui_scale);
        app.update();
        assert_eq!(app.world().resource::<UiScale>().0, 1.5);
    }
}
