//! Specimen columns: one per candidate family, each with weights, sizes and numerals.

use bevy::prelude::*;
use bevy::text::{FontFeatureTag, FontFeatures};

pub(super) fn spawn_candidate_column(
    root: &mut ChildSpawnerCommands,
    family: &str,
    base: f32,
    fg: Color,
    dim: Color,
) {
    root.spawn(Node {
        flex_direction: FlexDirection::Column,
        width: Val::Px(760.0),
        row_gap: Val::Px(2.0),
        ..default()
    })
    .with_children(|col| {
        line(col, family, family, 700, base + 4.0, dim, false);
        for (w, name) in [
            (400, "Regular 400"),
            (500, "Medium 500"),
            (600, "SemiBold 600"),
            (700, "Bold 700"),
        ] {
            line(
                col,
                &format!("{name}: Fahrplan München Äpfel Öl Übung ß € – Next stop: Hauptbahnhof"),
                family,
                w,
                base,
                fg,
                false,
            );
        }
        for size in [12.0, 13.0, 16.0, 20.0] {
            line(
                col,
                &format!(
                    "{size:.0} px  Start new drive · Settings · Quit — 07:45 → 08:12 — 48 km/h"
                ),
                family,
                400,
                size,
                fg,
                false,
            );
        }
        line(
            col,
            "Numerals 1111111111 / 0000000000 / 8888888888 (default)",
            family,
            500,
            base + 2.0,
            fg,
            false,
        );
        line(
            col,
            "Numerals 1111111111 / 0000000000 / 8888888888 (tnum)",
            family,
            500,
            base + 2.0,
            fg,
            true,
        );
        line(
            col,
            "48 km/h  12:45  1,234.56 m  il1I O0 rn m",
            family,
            600,
            base + 14.0,
            fg,
            true,
        );
    });
}
pub(super) fn line(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    family: &str,
    weight: u16,
    size: f32,
    color: Color,
    tnum: bool,
) {
    let mut font = TextFont {
        font: FontSource::Family(family.into()),
        font_size: FontSize::Px(size),
        weight: FontWeight(weight),
        ..default()
    };
    if tnum {
        font.font_features = FontFeatures::builder()
            .enable(FontFeatureTag::TABULAR_FIGURES)
            .build();
    }
    parent.spawn((Text::new(text), font, TextColor(color)));
}
