//! Where the instances of an attachment lie along its carrier's chain.

use std::collections::HashSet;

use super::*;

/// Where one instance of an attachment lies.
pub struct InstanceSite<'a> {
    /// Instance index `k`.
    pub k: u32,
    /// Chainage from the start of the carrier's chain (m).
    pub chainage: f64,
    /// The spline it lies on, with its tile.
    pub spline: &'a Placed,
    /// Arc length on that spline (m).
    pub local: f64,
}

/// The instances one attachment places, in instance order.
///
/// A single attachment (no run) is located along its chain wherever that leads, as before; a run
/// places the instances from its first index on along the chain from its carrier, while the chain
/// stays on the carrier's tile.
#[must_use]
pub fn instance_sites<'a>(
    att: &MapSplineAttachment,
    carrier: &MapSplineInstance,
    index: &'a SplineIndex,
) -> Vec<InstanceSite<'a>> {
    let range = att.instances();
    let offset = index.chain_offset(carrier.id_code);
    if !att.is_run() {
        let k = *range.start();
        let chainage = att.chainage(k);
        return index
            .locate(carrier.id_code, chainage - offset)
            .map(|(spline, local)| InstanceSite {
                k,
                chainage,
                spline,
                local,
            })
            .into_iter()
            .collect();
    }
    let Some(home) = index.by_id.get(&carrier.id_code).map(|(key, _)| *key) else {
        return Vec::new();
    };
    let (mut k, last) = (*range.start(), *range.end());
    let mut out = Vec::new();
    let mut acc = offset;
    let mut cur = carrier.id_code;
    let mut seen = HashSet::new();
    while k <= last && seen.insert(cur) && seen.len() <= MAX_CHAIN {
        let Some(placed) = index.by_id.get(&cur) else {
            break;
        };
        if placed.0 != home {
            break;
        }
        let len = f64::from(placed.1.length_m.max(0.0));
        while k <= last {
            let chainage = att.chainage(k);
            if chainage > acc + len + CHAIN_EPS_M {
                break;
            }
            let local = chainage - acc;
            if local >= -CHAIN_EPS_M {
                out.push(InstanceSite {
                    k,
                    chainage,
                    spline: placed,
                    local: local.clamp(0.0, len),
                });
            }
            k += 1;
        }
        acc += len;
        cur = placed.1.conn_id_2;
    }
    out
}
