//! Colours of the HUD instruments from the **instrument token set** (`MaterialTheme::instruments`).
//!
//! Instrument entities carry a marker ([`InstrumentPanel`] for the translucent panel behind the
//! readouts, [`InstrumentInk`] for text) instead of a colour; [`sync_instrument_tokens`] paints
//! them from the palette of the current [`InstrumentVariant`] and repaints on a variant or theme
//! change. [`update_instrument_variant`] switches the variant by the sky's time of day, so the
//! night palette never outshines a night scene.

use bevy::prelude::*;
use opts_gui::theme::{InstrumentVariant, MaterialTheme};

/// Marks the translucent panel behind a group of instruments.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct InstrumentPanel;

/// Which colour of the instrument palette a text uses.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentInk {
    /// Primary readout text.
    Text,
    /// Captions and units.
    TextDim,
    /// Needle-style highlight (the speed value).
    Needle,
    /// Normal state.
    Ok,
    /// Warning state.
    Warning,
    /// Alarm state.
    Alarm,
}

/// Opacity factor of an instrument text or panel (the fade of the status bar, hints and labels).
/// Absent means fully opaque.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct InstrumentFade(pub f32);

fn faded(color: Color, fade: Option<&Ref<InstrumentFade>>) -> Color {
    match fade {
        Some(f) if f.0 < 1.0 => {
            let a = color.alpha();
            color.with_alpha(a * f.0.clamp(0.0, 1.0))
        }
        _ => color,
    }
}

/// Paints instrument panels and texts from the active palette.
#[allow(clippy::type_complexity)]
pub fn sync_instrument_tokens(
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    mut texts: Query<(
        Ref<InstrumentInk>,
        Option<Ref<InstrumentFade>>,
        &mut TextColor,
    )>,
    mut panels: Query<(
        Ref<InstrumentPanel>,
        Option<Ref<InstrumentFade>>,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    let changed = theme.is_changed() || variant.is_changed();
    let palette = theme.instrument(*variant);
    for (ink, fade, mut color) in &mut texts {
        if changed || ink.is_changed() || fade.as_ref().is_some_and(Ref::is_changed) {
            let base = match *ink {
                InstrumentInk::Text => palette.text,
                InstrumentInk::TextDim => palette.text_dim,
                InstrumentInk::Needle => palette.needle,
                InstrumentInk::Ok => palette.ok,
                InstrumentInk::Warning => palette.warning,
                InstrumentInk::Alarm => palette.alarm,
            };
            color.0 = faded(base, fade.as_ref());
        }
    }
    for (panel, fade, mut bg, mut border) in &mut panels {
        if changed || panel.is_added() || fade.as_ref().is_some_and(Ref::is_changed) {
            bg.0 = faded(palette.panel, fade.as_ref());
            *border = BorderColor::all(faded(palette.panel_border, fade.as_ref()));
        }
    }
}

/// Picks the instrument palette from the session environment: night when vehicle lights are
/// needed (sun lower than 3 degrees), day otherwise (also without an environment).
pub fn update_instrument_variant(
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    mut variant: ResMut<InstrumentVariant>,
) {
    let wanted = match env {
        Some(env) if env.night() => InstrumentVariant::Night,
        _ => InstrumentVariant::Day,
    };
    if *variant != wanted {
        *variant = wanted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_and_panels_follow_the_variant() {
        let mut app = App::new();
        app.insert_resource(MaterialTheme::dark())
            .insert_resource(InstrumentVariant::Day)
            .add_systems(Update, sync_instrument_tokens);
        let t = app
            .world_mut()
            .spawn((InstrumentInk::Text, TextColor(Color::BLACK)))
            .id();
        let p = app
            .world_mut()
            .spawn((
                InstrumentPanel,
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
            ))
            .id();
        app.update();
        let day = app.world().get::<TextColor>(t).map(|c| c.0);
        assert_eq!(day, Some(MaterialTheme::dark().instruments.day.text));
        *app.world_mut().resource_mut::<InstrumentVariant>() = InstrumentVariant::Night;
        app.update();
        let night = app.world().get::<TextColor>(t).map(|c| c.0);
        assert_eq!(night, Some(MaterialTheme::dark().instruments.night.text));
        assert_eq!(
            app.world().get::<BackgroundColor>(p).map(|b| b.0),
            Some(MaterialTheme::dark().instruments.night.panel)
        );
    }

    #[test]
    fn an_alarm_ink_uses_the_alarm_colour() {
        let mut app = App::new();
        app.insert_resource(MaterialTheme::dark())
            .insert_resource(InstrumentVariant::Day)
            .add_systems(Update, sync_instrument_tokens);
        let t = app
            .world_mut()
            .spawn((InstrumentInk::Ok, TextColor(Color::BLACK)))
            .id();
        app.update();
        *app.world_mut().get_mut::<InstrumentInk>(t).expect("ink") = InstrumentInk::Alarm;
        app.update();
        assert_eq!(
            app.world().get::<TextColor>(t).map(|c| c.0),
            Some(MaterialTheme::dark().instruments.day.alarm)
        );
    }
}
