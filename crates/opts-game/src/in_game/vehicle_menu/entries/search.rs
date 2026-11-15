//! The vehicle search: the words a vehicle must match and the typing into the search field.

use super::*;

/// Whether a vehicle matches every word of the search text (name, id, make, model, repaint).
pub(in crate::in_game::vehicle_menu) fn matches_search(v: &VehicleItem, search: &str) -> bool {
    let hay = format!(
        "{} {} {} {} {}",
        v.full_name, v.id, v.manufacturer, v.model, v.default_livery
    )
    .to_lowercase();
    search
        .to_lowercase()
        .split_whitespace()
        .all(|w| hay.contains(w))
}

/// Typing into the vehicle search field (while the Select Vehicle page is up).
pub(in crate::in_game::vehicle_menu) fn search_input(
    mut keys: MessageReader<KeyboardInput>,
    mut menu: ResMut<VehicleMenu>,
    mut commands_in: ResMut<MenuCommands>,
) {
    // A configuration being renamed takes the keys (the name field of the Configurations page).
    if menu.page == Page::Configs && menu.configs.renaming.is_some() {
        for e in keys.read() {
            if e.state != ButtonState::Pressed {
                continue;
            }
            let Some((_, text)) = menu.configs.renaming.as_mut() else {
                break;
            };
            if e.logical_key == Key::Backspace {
                text.pop();
            } else if e.logical_key == Key::Enter {
                commands_in.0.push("commitrename".into());
            } else if let Some(t) = &e.text {
                text.extend(t.chars().filter(|c| !c.is_control()));
            }
        }
        // `menu.do typename <text>` sets the field (for scripts).
        let mut typed = None;
        commands_in
            .0
            .retain(|line| match line.strip_prefix("typename") {
                Some(rest) => {
                    typed = Some(rest.trim().to_string());
                    false
                }
                None => true,
            });
        if let (Some(t), Some((_, text))) = (typed, menu.configs.renaming.as_mut()) {
            *text = t;
        }
        return;
    }
    let on_route = menu.page == Page::Route;
    let typing = (menu.page == Page::Select && !menu.config_open) || on_route;
    let mut changed = false;
    let mut search = if on_route {
        menu.route_search.clone()
    } else {
        menu.search.clone()
    };
    for e in keys.read() {
        if !typing || e.state != ButtonState::Pressed {
            continue;
        }
        if e.logical_key == Key::Backspace {
            changed |= search.pop().is_some();
        } else if let Some(t) = &e.text {
            for ch in t.chars().filter(|c| !c.is_control()) {
                search.push(ch);
                changed = true;
            }
        }
    }
    // `menu.do search <text>` sets the field (an empty text clears it).
    let mut remote = Vec::new();
    commands_in.0.retain(|line| {
        if let Some(rest) = line.strip_prefix("search") {
            remote.push(rest.trim().to_string());
            false
        } else {
            true
        }
    });
    for text in remote {
        search = text;
        changed = true;
    }
    if on_route {
        if changed && search != menu.route_search {
            menu.route_search = search;
            menu.scroll[5] = 0.0;
        }
    } else if changed && search != menu.search {
        menu.search = search;
        menu.scroll[0] = 0.0;
    }
}
