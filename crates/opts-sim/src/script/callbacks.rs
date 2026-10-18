//! The engine callbacks scripts reach with `(M.V.name)`: their stack signatures, the host
//! handler interface and the stack-correct stub for every callback the host does not implement
//! (`docs/src/architecture/osc-runtime.md`, "The Host Interface").
//!
//! The registry, not the host, moves the stacks: the VM pops the arguments a signature names,
//! hands them to the handler and pushes exactly the results the signature names. A callback the
//! host does not implement therefore never misaligns the stacks; it answers with neutral values
//! and is reported once per engine. Signatures come from the callbacks reference
//! (`docs/src/formats/text/vehicles/callbacks.md`) and OMSI's shipped
//! `program/callbacklist_*.txt`.

use std::collections::BTreeMap;

/// The float value a stub pushes for a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neutral {
    /// `0`: counts, lengths, times, colours, heights.
    Zero,
    /// `-1`: indices, where the scripts test for "not found".
    NotFound,
}

/// Stack effect of one callback: arguments bottom to top, then results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallbackSignature {
    /// Lower-case name.
    pub name: &'static str,
    /// Floats popped (the last pushed by the script is the last of the arguments).
    pub float_args: u8,
    /// Strings popped.
    pub string_args: u8,
    /// Floats pushed.
    pub float_results: u8,
    /// Strings pushed.
    pub string_results: u8,
    /// What a stub pushes for each float result.
    pub neutral: Neutral,
    /// The signature is inferred with high confidence (arity knee and result type clear).
    pub confident: bool,
}

const fn sig(
    name: &'static str,
    float_args: u8,
    string_args: u8,
    float_results: u8,
    string_results: u8,
    neutral: Neutral,
    confident: bool,
) -> CallbackSignature {
    CallbackSignature {
        name,
        float_args,
        string_args,
        float_results,
        string_results,
        neutral,
        confident,
    }
}

/// Every callback of OMSI's lists (`callbacklist_roadvehicle.txt` 25, `callbacklist_scripttex.txt`
/// 17, `callbacklist_scenobj.txt` 3), with the signature of the callbacks reference. The three
/// script-texture calls no installed script uses and the three scenery callbacks carry assumed
/// signatures (`confident == false`).
pub const CALLBACKS: &[CallbackSignature] = &[
    // Route and timetable (.hof tables, active timetable)
    sig("getrouteindex", 1, 0, 1, 0, Neutral::NotFound, true),
    sig("getbusstopstring", 2, 0, 0, 1, Neutral::Zero, true),
    sig("getterminusstring", 2, 0, 0, 1, Neutral::Zero, true),
    sig("getrouteterminusindex", 1, 0, 1, 0, Neutral::NotFound, true),
    sig("getterminusindex", 1, 0, 1, 0, Neutral::NotFound, true),
    sig("getterminuscode", 1, 0, 1, 0, Neutral::NotFound, true),
    sig("getbusstopcount", 1, 0, 1, 0, Neutral::Zero, true),
    sig("getroutebusstopident", 2, 0, 0, 1, Neutral::Zero, true),
    sig("getbusstopindex", 0, 1, 1, 0, Neutral::NotFound, true),
    sig("getdepotstringglobal", 1, 0, 0, 1, Neutral::Zero, true),
    sig("getttbusstopcount", 0, 0, 1, 0, Neutral::Zero, true),
    sig("getttbusstopindex", 0, 0, 1, 0, Neutral::Zero, true),
    sig("getttterminusindex", 0, 0, 1, 0, Neutral::NotFound, true),
    sig("getttlinestring", 0, 0, 0, 1, Neutral::Zero, true),
    sig("getttdelay", 0, 0, 1, 0, Neutral::Zero, true),
    sig("getttbusstopname", 1, 0, 0, 1, Neutral::Zero, true),
    sig("getttbusstopdep", 1, 0, 1, 0, Neutral::Zero, true),
    sig("getttbusstoparr", 1, 0, 1, 0, Neutral::Zero, true),
    // Tickets and cash
    sig("getticketname", 1, 0, 0, 1, Neutral::Zero, true),
    sig("getticketvalue", 1, 0, 1, 0, Neutral::Zero, true),
    sig("givechangecoin", 1, 0, 0, 0, Neutral::Zero, true),
    // Passengers, randomness, geometry
    sig("gethumancountonpathlink", 1, 0, 1, 0, Neutral::Zero, true),
    sig("gethumancountonseat", 1, 0, 1, 0, Neutral::Zero, true),
    sig("nrspecrandom", 1, 0, 1, 0, Neutral::Zero, true),
    sig("getheightabovepoint", 3, 0, 1, 0, Neutral::Zero, false),
    // Script textures
    sig("stnewtex", 1, 0, 0, 0, Neutral::Zero, true),
    sig("stlock", 1, 0, 0, 0, Neutral::Zero, true),
    sig("stunlock", 1, 0, 0, 0, Neutral::Zero, true),
    sig("stfilter", 1, 0, 0, 0, Neutral::Zero, true),
    sig("stsetcolor", 5, 0, 0, 0, Neutral::Zero, false),
    sig("stdrawpixel", 3, 0, 0, 0, Neutral::Zero, false),
    sig("stdrawrect", 5, 0, 0, 0, Neutral::Zero, true),
    sig("sttextout", 6, 1, 0, 0, Neutral::Zero, false),
    sig("getfontindex", 0, 1, 1, 0, Neutral::Zero, true),
    sig("textlength", 1, 1, 1, 0, Neutral::Zero, true),
    sig("streadpixel", 3, 0, 0, 0, Neutral::Zero, false),
    sig("stcopycolor", 1, 0, 0, 0, Neutral::Zero, false),
    sig("stloadtex", 1, 1, 0, 0, Neutral::Zero, false),
    sig("stgetr", 1, 0, 1, 0, Neutral::Zero, true),
    sig("stgetg", 1, 0, 1, 0, Neutral::Zero, true),
    sig("stgetb", 1, 0, 1, 0, Neutral::Zero, true),
    sig("stgeta", 1, 0, 1, 0, Neutral::Zero, false),
    // Scenery objects (bus-stop displays)
    sig("getarrbusline", 1, 0, 0, 1, Neutral::Zero, false),
    sig("getarrbusterminus", 1, 0, 0, 1, Neutral::Zero, false),
    sig("getarrbustimediff", 1, 0, 1, 0, Neutral::Zero, false),
];

/// The signature of callback `name` (any case), if it is one of OMSI's.
#[must_use]
pub fn signature(name: &str) -> Option<&'static CallbackSignature> {
    let lower = name.to_ascii_lowercase();
    CALLBACKS.iter().find(|s| s.name == lower)
}

/// Arguments of one call, in the order the signature names them (`floats[0]` is the first
/// argument, pushed first by the script).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CallbackArgs {
    /// Float arguments.
    pub floats: Vec<f32>,
    /// String arguments.
    pub strings: Vec<String>,
}

impl CallbackArgs {
    /// Float argument `i`, or 0.
    #[must_use]
    pub fn f(&self, i: usize) -> f32 {
        self.floats.get(i).copied().unwrap_or(0.0)
    }

    /// Float argument `i` as a non-negative index.
    #[must_use]
    pub fn index(&self, i: usize) -> usize {
        self.f(i).max(0.0) as usize
    }

    /// String argument `i`, or empty.
    #[must_use]
    pub fn s(&self, i: usize) -> &str {
        self.strings.get(i).map_or("", String::as_str)
    }
}

/// Results of one call: pushed in order (the last float becomes the top of the stack).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CallbackResult {
    /// Float results.
    pub floats: Vec<f32>,
    /// String results.
    pub strings: Vec<String>,
}

impl CallbackResult {
    /// No result (`GiveChangeCoin`, the canvas calls).
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// One float result.
    #[must_use]
    pub fn float(v: f32) -> Self {
        Self {
            floats: vec![v],
            strings: Vec::new(),
        }
    }

    /// One string result.
    #[must_use]
    pub fn string(s: impl Into<String>) -> Self {
        Self {
            floats: Vec::new(),
            strings: vec![s.into()],
        }
    }
}

impl CallbackSignature {
    /// Answers a call with the stub: the arguments leave the stacks (as far as they are there)
    /// and the neutral results arrive.
    pub fn apply_stub(&self, st: &mut super::strings::Stacks) {
        for _ in 0..self.float_args {
            st.numbers.pop();
        }
        for _ in 0..self.string_args {
            st.strings.pop();
        }
        let neutral = self.neutral_result();
        st.numbers.extend(neutral.floats);
        st.strings.extend(neutral.strings);
    }

    /// The stub's answer: neutral floats and empty strings.
    #[must_use]
    pub fn neutral_result(&self) -> CallbackResult {
        let v = match self.neutral {
            Neutral::Zero => 0.0,
            Neutral::NotFound => -1.0,
        };
        CallbackResult {
            floats: vec![v; usize::from(self.float_results)],
            strings: vec![String::new(); usize::from(self.string_results)],
        }
    }
}

/// The host's implementation of the callbacks: receives the lower-case name and the popped
/// arguments; returns `None` for a callback it does not implement (the registry then answers
/// with the stub and reports it once).
pub type CallbackHandler = Box<dyn FnMut(&str, &CallbackArgs) -> Option<CallbackResult> + Send>;

/// How often each callback was answered, by name: by the host, by the stub, or unknown to the
/// registry (no signature: nothing popped or pushed).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallbackStats {
    /// Calls the host implemented.
    pub real: BTreeMap<String, u64>,
    /// Calls the stub answered.
    pub stub: BTreeMap<String, u64>,
    /// Calls to names outside OMSI's lists.
    pub unknown: BTreeMap<String, u64>,
}

impl CallbackStats {
    /// Names of the callbacks the stub answered.
    #[must_use]
    pub fn stubbed(&self) -> Vec<String> {
        self.stub.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
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
        use super::super::strings::Stacks;
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
}
