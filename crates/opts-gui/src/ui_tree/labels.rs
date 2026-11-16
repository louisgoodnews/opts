//! The accessible name of an element and the visible text below it.

use bevy::prelude::*;

use super::A11yHidden;
use crate::components::{text_area::TextArea, text_field::M3TextField, toast::M3Toast};
use crate::interaction::tooltip::Tooltip;

/// Visible text below `entity` (depth-first, joined by one space).
pub fn text_below(world: &World, entity: Entity) -> String {
    let mut parts = Vec::new();
    collect_text(world, entity, &mut parts, 0);
    parts.join(" ")
}

fn collect_text(world: &World, entity: Entity, out: &mut Vec<String>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(e) = world.get_entity(entity) else {
        return;
    };
    let icon_font = e.get::<TextFont>().is_some_and(
        |f| matches!(&f.font, bevy::text::FontSource::Handle(h) if *h == crate::fonts::ICON_FONT),
    );
    if e.contains::<A11yHidden>() || icon_font {
        return;
    }
    if let Some(text) = e.get::<Text>() {
        if !text.0.trim().is_empty() {
            out.push(text.0.trim().to_string());
        }
    }
    if let Some(children) = e.get::<Children>() {
        for child in children.iter() {
            collect_text(world, child, out, depth + 1);
        }
    }
}

/// The accessible name of an element.
pub fn label_of(world: &World, entity: Entity) -> String {
    let Ok(e) = world.get_entity(entity) else {
        return String::new();
    };
    if let Some(field) = e.get::<M3TextField>() {
        return if field.label.is_empty() {
            field.placeholder.clone()
        } else {
            field.label.clone()
        };
    }
    if let Some(area) = e.get::<TextArea>() {
        return area.placeholder.clone();
    }
    if let Some(toast) = e.get::<M3Toast>() {
        return toast.message.clone();
    }
    let text = text_below(world, entity);
    if text.is_empty() {
        e.get::<Tooltip>()
            .map(|t| t.0.to_string())
            .unwrap_or_default()
    } else {
        text
    }
}
