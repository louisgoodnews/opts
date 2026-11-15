//! Unit tests of the callback signatures and the stubs.

use super::*;

#[test]
fn every_omsi_callback_has_exactly_one_signature() {
    assert_eq!(CALLBACKS.len(), 45);
    let mut names: Vec<_> = CALLBACKS.iter().map(|s| s.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 45);
    assert!(CALLBACKS.iter().all(|s| s.name == s.name.to_lowercase()));
    assert_eq!(
        signature("GetTerminusString").map(|s| s.float_args),
        Some(2)
    );
    assert!(signature("nothing").is_none());
}

#[test]
fn every_stub_removes_its_arguments_and_adds_its_results_and_nothing_else() {
    use crate::script::strings::Stacks;
    for sig in CALLBACKS {
        let mut st = Stacks::default();
        // A marker below the arguments must survive.
        st.numbers.push(99.0);
        st.strings.push("below".into());
        for _ in 0..sig.float_args {
            st.numbers.push(5.0);
        }
        for _ in 0..sig.string_args {
            st.strings.push("x".into());
        }
        sig.apply_stub(&mut st);
        assert_eq!(
            st.numbers.len(),
            1 + usize::from(sig.float_results),
            "{} floats",
            sig.name
        );
        assert_eq!(
            st.strings.len(),
            1 + usize::from(sig.string_results),
            "{} strings",
            sig.name
        );
        assert_eq!((st.numbers[0], st.strings[0].as_str()), (99.0, "below"));
    }
}

#[test]
fn the_stub_answers_with_neutral_values_of_the_right_shape() {
    let r = signature("GetRouteIndex").unwrap().neutral_result();
    assert_eq!(r, CallbackResult::float(-1.0));
    let r = signature("GetTerminusString").unwrap().neutral_result();
    assert_eq!(r, CallbackResult::string(""));
    let r = signature("STTextOut").unwrap().neutral_result();
    assert_eq!(r, CallbackResult::none());
}
