//! The record of one object before any check has run.

use super::*;

pub(super) fn base_record(entry: &ObjectEntry, pre: &Preflight, worker: usize) -> Record {
    let (status, channels) = script_check::evaluate(pre);
    let sound = pre.object.as_ref().and_then(|o| o.sound.clone());
    let resolves = sound.as_ref().map(|s| {
        pre.path
            .as_ref()
            .and_then(|p| p.parent())
            .is_some_and(|d| d.join(s.replace('\\', "/")).exists())
    });
    Record {
        sco: entry.sco.clone(),
        key: entry.key.clone(),
        tier: entry.tier.clone(),
        class: pre.classes.clone(),
        tags: Tags {
            unsupported: pre.unsupported(),
            script: status,
            effect_channels: channels,
            info: Vec::new(),
        },
        audio: Audio {
            status: "recorded".into(),
            sound_cfg: sound,
            resolves,
            triggers: Vec::new(),
        },
        worker,
        ..Record::default()
    }
}
