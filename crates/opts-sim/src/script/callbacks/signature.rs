//! The stack effect of a callback and the neutral answer a stub gives for it.

use super::args::CallbackResult;
use crate::script::strings::Stacks;

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

impl CallbackSignature {
    /// Answers a call with the stub: the arguments leave the stacks (as far as they are there)
    /// and the neutral results arrive.
    pub fn apply_stub(&self, st: &mut Stacks) {
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
