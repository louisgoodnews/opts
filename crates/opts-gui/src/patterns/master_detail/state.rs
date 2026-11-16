//! The configuration, entities and internal state of one master-detail pattern instance.

use bevy::prelude::*;

use super::model::Chip;
use crate::patterns::list_model::{Row, RowLayout};

/// Static settings of one pattern instance.
#[derive(Debug, Clone, Copy)]
pub struct MasterDetailConfig {
    /// Prefix of the `UiId`s (`wizard.maps`).
    pub prefix: &'static str,
    /// Message key shown instead of the list when the model has no entries at all.
    pub empty_key: &'static str,
    /// Message key of the search placeholder (variable `count`: the number of entries).
    pub search_key: &'static str,
}

/// The entities of one pattern instance.
#[derive(Debug, Clone, Copy)]
pub struct MasterDetailParts {
    /// Root node: put it where the pattern belongs.
    pub root: Entity,
    /// Container of the detail panel: fill it on [`super::messages::MasterDetailSelected`].
    pub detail: Entity,
    /// The list (single focus stop).
    pub list: Entity,
    /// The search field (frame).
    pub search: Entity,
    pub(in crate::patterns) list_col: Entity,
    pub(in crate::patterns) chips: Entity,
    pub(in crate::patterns) content: Entity,
    pub(in crate::patterns) notice: Entity,
    pub(in crate::patterns) notice_text: Entity,
    pub(in crate::patterns) track: Entity,
    pub(in crate::patterns) thumb: Entity,
}

/// Internal state of one pattern instance (rows, scroll, what is currently drawn).
#[derive(Component, Debug)]
pub struct MasterDetailView {
    pub(in crate::patterns) parts: MasterDetailParts,
    pub(in crate::patterns) config: MasterDetailConfig,
    pub(in crate::patterns) rows: Vec<Row>,
    pub(in crate::patterns) layout: RowLayout,
    pub(in crate::patterns) scroll: f32,
    pub(in crate::patterns) cursor: Option<usize>,
    pub(in crate::patterns) viewport: f32,
    pub(in crate::patterns) row_entities: Vec<Entity>,
    pub(in crate::patterns) rendered: Option<Rendered>,
    pub(in crate::patterns) rendered_chips: Option<(Vec<Chip>, Vec<String>)>,
    pub(in crate::patterns) chip_entities: Vec<Entity>,
    pub(in crate::patterns) version: u64,
    pub(in crate::patterns) announced: Option<Option<String>>,
    pub(in crate::patterns) reveal_pending: bool,
    pub(in crate::patterns) typed: String,
    pub(in crate::patterns) typed_at: f32,
    pub(in crate::patterns) last_click: Option<(String, f32)>,
}

/// What the row entities currently show (change detection of the renderer).
#[derive(Debug, Clone, PartialEq)]
pub(in crate::patterns) struct Rendered {
    pub range: std::ops::Range<usize>,
    pub version: u64,
    pub selected: Option<String>,
    pub favourites: Vec<String>,
    pub chars: usize,
}
