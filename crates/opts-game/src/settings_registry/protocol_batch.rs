//! Batch methods of the settings protocol (M-0072): `settings.set_many` (atomic),
//! `settings.import`, `settings.export` and `settings.diff` against the defaults.
//!
//! A batch is validated as a whole first, on a scratch copy of the settings: if one entry is
//! refused (unknown key, unavailable setting, bad value, a key binding that clashes with another
//! in the same batch) nothing is applied and the answer lists every refusal. Risky settings are
//! applied at once, without the confirmation of `settings.set confirm:false`.

mod apply;
mod export;

pub(crate) use apply::{import, set_many};
pub(crate) use export::{diff, export};

use bevy::prelude::*;
use opts_cli::protocol::RpcError;
use serde_json::{Value};

use super::protocol::{text_of};
use super::{
    apply_change_with, normalize, Apply, ChangeSource, SettingKind, SettingsCapabilities,
    SettingsRegistry,
};

/// Most entries one batch accepts.
const MAX_ENTRIES: usize = 1000;

/// The `(key, stored text)` entries of the object parameter `field`, in key order.
fn entries(params: &Value, field: &str) -> Result<Vec<(String, String)>, RpcError> {
    let Some(object) = params.get(field).and_then(Value::as_object) else {
        return Err(RpcError::invalid_params(format!(
            "parameter `{field}` must be an object of setting key to value"
        )));
    };
    if object.len() > MAX_ENTRIES {
        return Err(RpcError::invalid_params(format!(
            "at most {MAX_ENTRIES} settings per call"
        )));
    }
    let mut out = Vec::with_capacity(object.len());
    for (key, value) in object {
        let text = text_of(value).ok_or_else(|| {
            RpcError::invalid_params(format!(
                "`{key}`: value must be a string, number or boolean"
            ))
        })?;
        out.push((key.clone(), text));
    }
    out.sort();
    Ok(out)
}
