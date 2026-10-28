//! Unit tests for `super` (`registry.rs`).

use super::*;
use crate::save::container::SaveMeta;
use serde_json::json;

#[allow(clippy::unnecessary_wraps)]
fn noop_capture(_: &mut World) -> Result<Option<Value>, String> {
    Ok(Some(json!({"v": 2})))
}
fn noop_apply(_: &mut World, _: &Value) -> Result<(), String> {
    Ok(())
}
fn up(from: u32, data: Value) -> Result<Value, String> {
    if from == 1 {
        Ok(json!({"v": data["old"].as_i64().unwrap_or(0) + 1}))
    } else {
        Err("unknown version".into())
    }
}

fn section(id: &'static str, version: u32, migrate: Option<MigrateFn>) -> SaveSection {
    SaveSection {
        id,
        version,
        stage: Stage::World,
        order: 0,
        required: false,
        capture: noop_capture,
        prepare: None,
        apply: Some(noop_apply),
        migrate,
        check: None,
        about: "",
    }
}

fn file_with(sections: &[(&str, u32, Value)]) -> SaveFile {
    let mut f = SaveFile::new(SaveMeta::default());
    for (id, v, d) in sections {
        f.sections.insert(
            (*id).to_owned(),
            SectionBlob {
                version: *v,
                data: d.clone(),
            },
        );
    }
    f
}

#[test]
fn the_plan_migrates_old_sections_and_sets_the_rest_aside() {
    let sections = [
        section("same", 2, None),
        section("old", 2, Some(up)),
        section("old_no_migration", 2, None),
        section("too_new", 2, Some(up)),
        section("absent", 1, None),
        section("broken", 5, Some(up)),
    ];
    let file = file_with(&[
        ("same", 2, json!({"v": 7})),
        ("old", 1, json!({"old": 4})),
        ("old_no_migration", 1, json!({})),
        ("too_new", 3, json!({})),
        ("broken", 4, json!({})),
        ("from_the_future", 1, json!({"x": 1})),
    ]);
    let plan = plan_restore(&sections, &file);
    let applied: Vec<_> = plan.steps.iter().map(|(s, d)| (s.id, d.clone())).collect();
    assert_eq!(
        applied,
        vec![("same", json!({"v": 7})), ("old", json!({"v": 5}))]
    );
    assert_eq!(plan.unknown, ["from_the_future"]);
    assert_eq!(plan.missing, ["absent"]);
    assert_eq!(plan.warnings.len(), 3);
}

#[test]
fn steps_follow_stage_then_order() {
    let mut a = section("a", 1, None);
    a.stage = Stage::Settled;
    let b = section("b", 1, None);
    let mut c = section("c", 1, None);
    c.order = -1;
    let file = file_with(&[
        ("a", 1, json!({})),
        ("b", 1, json!({})),
        ("c", 1, json!({})),
    ]);
    let plan = plan_restore(&[a, b, c], &file);
    assert_eq!(
        plan.steps.iter().map(|(s, _)| s.id).collect::<Vec<_>>(),
        ["c", "b", "a"]
    );
}

#[test]
fn overwriting_keeps_what_this_build_does_not_understand() {
    let sections = [section("known", 1, None)];
    let old = file_with(&[
        ("known", 1, json!({"old": true})),
        ("future_section", 1, json!({"keep": 1})),
        ("newer_known", 9, json!({})),
    ]);
    let mut new = file_with(&[("known", 1, json!({"new": true}))]);
    carry_over_foreign(&sections, &old, &mut new);
    assert_eq!(new.sections["known"].data, json!({"new": true}));
    assert_eq!(new.sections["future_section"].data, json!({"keep": 1}));
    assert!(new.sections.contains_key("newer_known"));

    let sections = [section("known", 1, None), section("newer_known", 9, None)];
    let mut again = file_with(&[]);
    carry_over_foreign(&sections, &old, &mut again);
    assert!(
        !again.sections.contains_key("newer_known"),
        "known and not newer: not foreign"
    );
}
