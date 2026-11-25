//! The stand-in gear model: the engine speed and the gear of an AI vehicle from its speed, for a heavy vehicle and for a car.

/// The stand-in gear model of a vehicle class: idle and shift speed (rpm) and the speeds (km/h)
/// of the gear changes. A heavy vehicle idles at 650 rpm and shifts at 2,100 rpm at 12, 25, 40,
/// 60 and 90 km/h; a car idles at 800 rpm and shifts at 3,600 rpm at 18, 35, 55 and 85 km/h.
struct Gears {
    idle: f32,
    shift: f32,
    changes: &'static [f32],
}

const HEAVY: Gears = Gears {
    idle: 650.0,
    shift: 2100.0,
    changes: &[0.0, 12.0, 25.0, 40.0, 60.0, 90.0],
};
const LIGHT: Gears = Gears {
    idle: 800.0,
    shift: 3600.0,
    changes: &[0.0, 18.0, 35.0, 55.0, 85.0, 130.0],
};

/// Engine speed (rpm) of a road vehicle at `speed_kmh`: a gear model of its class, so the pitch
/// rises, drops at every shift and rises again. ❓ OMSI's AI computes `AI_Motor` itself; its model
/// is not known (Q-AU05), this is a stand-in with the right shape.
#[must_use]
pub fn ai_engine_rpm(speed_kmh: f32, heavy: bool) -> f32 {
    let g = if heavy { &HEAVY } else { &LIGHT };
    let v = speed_kmh.max(0.0);
    if v < 0.5 {
        return g.idle;
    }
    // Each gear covers from the previous change point (where it starts at 60 % of the shift
    // speed) to its own (the shift speed); the last one runs on at the same slope.
    let mut lo = 0.0;
    let mut hi = g.changes[1];
    for w in g.changes.windows(2) {
        lo = w[0];
        hi = w[1];
        if v < hi {
            break;
        }
    }
    let start = if lo == 0.0 { g.idle } else { g.shift * 0.62 };
    let f = ((v - lo) / (hi - lo)).max(0.0);
    (start + (g.shift - start) * f).min(g.shift * 1.5)
}

/// The gear of the stand-in model at `speed_kmh` (`0` below walking pace).
#[must_use]
pub fn ai_gear(speed_kmh: f32, heavy: bool) -> f32 {
    let g = if heavy { &HEAVY } else { &LIGHT };
    let past = g.changes[1..].iter().filter(|c| speed_kmh >= **c).count();
    if speed_kmh < 0.5 {
        0.0
    } else {
        (past + 1) as f32
    }
}
