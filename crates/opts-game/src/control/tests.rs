use super::*;
use crate::look::{ActiveLook, Look};

fn answer(outcome: Outcome) -> Result<Value, RpcError> {
    match outcome {
        Outcome::Done(r) => r,
        Outcome::Wait(_) => panic!("render.look answers immediately"),
    }
}

#[test]
fn render_look_gets_sets_and_rejects() {
    let mut world = World::new();
    world.insert_resource(ActiveLook::default());
    // get: Vivid is the default.
    let got = answer(render_look(&mut world, &json!({}))).expect("get");
    assert_eq!(got["look"], "vivid");
    assert_eq!(got["available"], json!(["vivid", "natural", "bevy"]));
    // set: takes effect on the resource.
    let set = answer(render_look(&mut world, &json!({ "look": "bevy" }))).expect("set");
    assert_eq!(
        (set["look"].as_str(), set["previous"].as_str()),
        (Some("bevy"), Some("vivid"))
    );
    assert_eq!(world.resource::<ActiveLook>().0, Look::BevyDefault);
    // unknown names are invalid parameters and change nothing.
    let err = answer(render_look(&mut world, &json!({ "look": "sepia" }))).expect_err("bad");
    assert_eq!(err.code, opts_cli::protocol::code::INVALID_PARAMS);
    assert_eq!(world.resource::<ActiveLook>().0, Look::BevyDefault);
}
