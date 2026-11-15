//! Scroll bar placement, the Compact / Wide layout adaptation and the shared `set_if` helper.

use bevy::prelude::*;

use crate::patterns::master_detail::MasterDetailView;
use crate::theme::spacing::LayoutClass;

pub(super) fn set_if<T: PartialEq>(slot: &mut T, value: T) {
    if *slot != value {
        *slot = value;
    }
}

pub(super) fn place_content(
    nodes: &mut Query<&mut Node>,
    view: &MasterDetailView,
    parts: crate::patterns::MasterDetailParts,
    height: f32,
) {
    let total = view.layout.total();
    if let Ok(mut n) = nodes.get_mut(parts.content) {
        set_if(&mut n.height, Val::Px(total));
        set_if(&mut n.top, Val::Px(-view.scroll));
    }
    let scrollable = total > height + 0.5;
    if let Ok(mut n) = nodes.get_mut(parts.track) {
        set_if(
            &mut n.display,
            if scrollable {
                Display::Flex
            } else {
                Display::None
            },
        );
    }
    if scrollable {
        let track = (height - 4.0).max(24.0);
        let thumb = (track * height / total).clamp(24.0, track);
        let max = view.layout.max_scroll(height).max(1.0);
        let top = (track - thumb) * (view.scroll / max);
        if let Ok(mut n) = nodes.get_mut(parts.thumb) {
            set_if(&mut n.height, Val::Px(thumb));
            set_if(&mut n.top, Val::Px(top));
        }
    }
}

/// Stacks the pattern in the Compact layout class and puts the list beside the detail otherwise.
pub(super) fn adapt_layout(
    class: Option<Res<LayoutClass>>,
    roots: Query<&MasterDetailView>,
    mut nodes: Query<&mut Node>,
) {
    let Some(class) = class else { return };
    let compact = *class == LayoutClass::Compact;
    for view in &roots {
        let p = view.parts;
        let root = p.root;
        if let Ok(mut n) = nodes.get_mut(root) {
            set_if(
                &mut n.flex_direction,
                if compact {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
            );
            set_if(
                &mut n.overflow,
                if compact {
                    Overflow::scroll_y()
                } else {
                    Overflow::visible()
                },
            );
        }
        if let Ok(mut n) = nodes.get_mut(p.list_col) {
            set_if(
                &mut n.width,
                if compact {
                    Val::Percent(100.0)
                } else {
                    Val::Percent(40.0)
                },
            );
            set_if(
                &mut n.min_width,
                if compact {
                    Val::Px(0.0)
                } else {
                    Val::Px(320.0)
                },
            );
            set_if(
                &mut n.height,
                if compact { Val::Px(260.0) } else { Val::Auto },
            );
            set_if(&mut n.flex_shrink, if compact { 0.0 } else { 1.0 });
        }
    }
}
