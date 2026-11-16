//! The outer edges of the drawn ground surfaces: the edges one surface triangle uses only.

use super::*;

type Key = (i32, i32, i32);

fn key(v: Vec3) -> Key {
    (
        (v.x * 100.0).round() as i32,
        (v.y * 100.0).round() as i32,
        (v.z * 100.0).round() as i32,
    )
}

/// The edges of `surfaces` that only one triangle uses (corners compared to a centimetre).
#[must_use]
pub fn boundary_edges(surfaces: &[GroundTriangle]) -> Vec<(Vec3, Vec3)> {
    boundary_edges_out(surfaces)
        .into_iter()
        .map(|(a, b, _)| (a, b))
        .collect()
}

/// The outer edges with the unit direction that points away from their triangle (outside).
#[must_use]
pub fn boundary_edges_out(surfaces: &[GroundTriangle]) -> Vec<(Vec3, Vec3, Vec2)> {
    let mut count: HashMap<(Key, Key), (u8, Vec3, Vec3, Vec3)> = HashMap::new();
    for tri in surfaces {
        for e in 0..3 {
            let (a, b, c) = (tri[e], tri[(e + 1) % 3], tri[(e + 2) % 3]);
            let (ka, kb) = (key(a), key(b));
            if ka == kb {
                continue;
            }
            let k = if ka < kb { (ka, kb) } else { (kb, ka) };
            let entry = count.entry(k).or_insert((0, a, b, c));
            entry.0 = entry.0.saturating_add(1);
        }
    }
    count
        .into_values()
        .filter(|(n, ..)| *n == 1)
        .filter_map(|(_, a, b, c)| {
            let ab = (b - a).truncate();
            let len = ab.length();
            if len < 1e-4 {
                return None;
            }
            let mut out = Vec2::new(-ab.y, ab.x) / len;
            if out.dot((c - a).truncate()) > 0.0 {
                out = -out;
            }
            Some((a, b, out))
        })
        .collect()
}
