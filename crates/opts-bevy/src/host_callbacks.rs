//! The callbacks OMSI offers its vehicle scripts (`(M.V.name)`), as far as OPTS implements them.
//!
//! The scripts call the host for data the scripts cannot know: random numbers fixed per vehicle,
//! the timetable and its termini, bus stops, tickets, passengers and fonts. Implemented so far:
//! `NrSpecRandom`. The rest (about 40 callbacks, see
//! `docs/src/formats/text/vehicles/predefined-variables.md`) is listed in the roadmap
//! (M-0057, M-0058); until then they are reported once per vehicle and leave the stack as it is.

use opts_sim::script::{ExternalHandler, Stacks};

/// SplitMix64 finaliser: a well-mixed 64-bit value from `x`.
fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Callbacks of one vehicle; `seed` identifies it, so its "random" door speeds and the like stay
/// the same for the whole session and differ between vehicles.
#[must_use]
pub fn vehicle_callbacks(seed: u64) -> ExternalHandler {
    Box::new(move |name, st: &mut Stacks| match name {
        // `n (M.V.NrSpecRandom)`: a number in [0, 1) fixed per vehicle and per `n`.
        "nrspecrandom" => {
            let n = st.numbers.pop().unwrap_or(0.0);
            let bits = mix(seed ^ mix(u64::from(n.round() as i64 as u64 as u32)));
            st.numbers.push((bits >> 40) as f32 / 16_777_216.0);
            true
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nr_spec_random_is_stable_per_vehicle_and_number() {
        let mut a = vehicle_callbacks(1);
        let mut b = vehicle_callbacks(2);
        let call = |h: &mut ExternalHandler, n: f32| {
            let mut st = Stacks::default();
            st.numbers.push(n);
            assert!(h("nrspecrandom", &mut st));
            st.numbers[0]
        };
        let (x, y) = (call(&mut a, 301.0), call(&mut a, 302.0));
        assert_eq!(x, call(&mut a, 301.0));
        assert_ne!(x, y);
        assert_ne!(x, call(&mut b, 301.0));
        assert!((0.0..1.0).contains(&x));
        assert!(!a("unknown", &mut Stacks::default()));
    }
}
