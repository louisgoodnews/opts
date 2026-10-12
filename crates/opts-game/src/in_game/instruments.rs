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

/// Paints instrument panels and texts from the active palette.
pub fn sync_instrument_tokens(
    theme: Res<MaterialTheme>,
    variant: Res<InstrumentVariant>,
    mut texts: Query<(Ref<InstrumentInk>, &mut TextColor)>,
    mut panels: Query<(Ref<InstrumentPanel>, &mut BackgroundColor, &mut BorderColor)>,
) {
    let changed = theme.is_changed() || variant.is_changed();
    let palette = theme.instrument(*variant);
    for (ink, mut color) in &mut texts {
        if changed || ink.is_changed() {
            color.0 = match *ink {
                InstrumentInk::Text => palette.text,
                InstrumentInk::TextDim => palette.text_dim,
                InstrumentInk::Needle => palette.needle,
                InstrumentInk::Ok => palette.ok,
                InstrumentInk::Warning => palette.warning,
                InstrumentInk::Alarm => palette.alarm,
            };
        }
    }
    for (panel, mut bg, mut border) in &mut panels {
        if changed || panel.is_added() {
            bg.0 = palette.panel;
            *border = BorderColor::all(palette.panel_border);
        }
    }
}

/// Picks the instrument palette from the session's time of day: night when the sky model says
/// vehicle lights are needed (sun lower than 3 degrees), day otherwise (also without a sky).
pub fn update_instrument_variant(
    sky: Option<Res<crate::in_game::sky::Sky>>,
    time: Res<Time<Virtual>>,
    mut variant: ResMut<InstrumentVariant>,
) {
    let wanted = match sky {
        Some(sky) if sky.lights_needed(time.elapsed_secs()) => InstrumentVariant::Night,
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
