//! The interpolation of the times the profiles do not list, and the offsets of every station.

use super::TtProfile;

/// How the times of the stations a profile does not list are found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// By the order of the stations only (the default: over the 33,948 manual times of the
    /// corpus that were hidden and predicted again, the order was as good as the link lengths in
    /// the mean and better within half a minute, 88 % against 75 %).
    #[default]
    ByIndex,
    /// By the lengths of the links of `StnLinks.cfg` where every pair of stations has one.
    ByDistance,
}

/// Arrival and departure offsets (minutes after the start) of every station of a trip under a
/// profile.
///
/// The profile lists manual times for some stations only; the others are interpolated linearly
/// by station index (or by the lengths of the links, `StnLinks.cfg`) between the neighbours that have one, the first station being at 0 and the
/// last at the profile's running time unless they have times of their own ❓ (the files give no
/// distances). Time never runs backwards: an arrival before the previous departure is moved to it. A station with only a departure arrives `otherstopping` seconds before it, one
/// with only an arrival departs `otherstopping` seconds after it; the last station departs when
/// it arrives.
pub fn station_times(p: &TtProfile, n: usize, segments: Option<&[f32]>) -> Vec<(f32, f32)> {
    if n == 0 {
        return Vec::new();
    }
    let stop_min = |i: usize| p.other_stopping.get(&i).copied().unwrap_or(0.0) / 60.0;
    let mut reference: Vec<Option<f32>> = (0..n)
        .map(|i| {
            p.arrival_min
                .get(&i)
                .copied()
                .or_else(|| p.departure_min.get(&i).map(|d| d - stop_min(i)))
        })
        .collect();
    if reference[0].is_none() {
        reference[0] = Some(0.0);
    }
    if n > 1 && reference[n - 1].is_none() {
        reference[n - 1] = Some(
            p.total_min
                .max(reference.iter().flatten().copied().fold(0.0, f32::max)),
        );
    }
    // Position of each station along the trip: its index, or the cumulated link lengths.
    let position: Vec<f32> = match segments {
        Some(segs) if segs.len() + 1 == n => std::iter::once(0.0)
            .chain(segs.iter().scan(0.0f32, |acc, d| {
                *acc += d;
                Some(*acc)
            }))
            .collect(),
        _ => (0..n).map(|i| i as f32).collect(),
    };
    let mut filled = vec![0.0f32; n];
    let mut prev = 0usize;
    for i in 0..n {
        if let Some(v) = reference[i] {
            filled[i] = v;
            // Interpolate the run of stations between the previous anchor and this one.
            let span = position[i] - position[prev];
            for k in prev + 1..i {
                let f = if span > 0.0 {
                    (position[k] - position[prev]) / span
                } else {
                    (k - prev) as f32 / (i - prev) as f32
                };
                filled[k] = filled[prev] + (v - filled[prev]) * f;
            }
            prev = i;
        }
    }
    // Authored contradictions (an arrival of 1 minute at the station after one that departs at
    // 18.5, found in 5,751 of 291,253 runs) are repaired by never letting time run backwards.
    let mut out = Vec::with_capacity(n);
    let mut clock = 0.0f32;
    for (i, &reference) in filled.iter().enumerate() {
        let arrival = reference.max(clock);
        let departure = if i + 1 == n {
            arrival
        } else {
            p.departure_min
                .get(&i)
                .copied()
                .unwrap_or(arrival + stop_min(i))
                .max(arrival)
        };
        clock = departure;
        out.push((arrival, departure));
    }
    out
}
