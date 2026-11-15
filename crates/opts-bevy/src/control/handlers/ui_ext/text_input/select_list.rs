//! Selecting an entry of a master list like a click on its row does (`ui.select`).

use super::super::*;

/// Selects an entry of a master list like a click on its row does (by id or title), and chooses
/// it with `choose`.
pub(super) fn select_in_list(
    world: &mut World,
    params: &Value,
    list: Entity,
    (id, option): (&str, &Option<String>),
) -> Result<Outcome, RpcError> {
    let key = option.clone().ok_or_else(|| {
        RpcError::invalid_params("`option` (entry id or title) is needed for a list")
    })?;
    let md = world
        .get::<MasterDetail>(list)
        .cloned()
        .ok_or_else(|| not_found(id))?;
    let entry = md
        .entries
        .iter()
        .find(|e| e.id == key)
        .or_else(|| md.entries.iter().find(|e| e.title.eq_ignore_ascii_case(&key)))
        .map(|e| e.id.clone())
        .ok_or_else(|| {
            RpcError::new(code::NOT_FOUND, format!("`{key}` is not an entry of `{id}`"))
                .with_data(json!({ "entries": md.entries.iter().take(200).map(|e| json!({"id": e.id, "title": e.title})).collect::<Vec<_>>() }))
        })?;
    let shown = {
        let mut md_live = world
            .get::<MasterDetail>(list)
            .cloned()
            .ok_or_else(|| not_found(id))?;
        let mut view = world
            .get_mut::<MasterDetailView>(list)
            .ok_or_else(|| not_found(id))?;
        let ok = view.select_entry(&mut md_live, &entry);
        if ok {
            *world
                .get_mut::<MasterDetail>(list)
                .ok_or_else(|| not_found(id))? = md_live;
        }
        ok
    };
    if !shown {
        return Err(RpcError::new(
            code::NOT_READY,
            format!("`{entry}` is filtered out of the list"),
        ));
    }
    if opt_bool(params, "choose")?.unwrap_or(false) {
        world.write_message(MasterDetailChosen {
            root: list,
            id: entry.clone(),
        });
    }
    Ok(Outcome::ok(
        json!({ "ok": true, "id": id, "selected": entry }),
    ))
}
