//! Turning the triangles of one model into expected triangles, and the tree billboards.

use super::*;

pub(super) fn add_model(
    tris: &mut Vec<ExpectedTri>,
    model: &O3d,
    mesh: usize,
    overrides: &[(TextureOverride, bool)],
    base: bool,
) {
    let plain: Vec<TextureOverride> = overrides.iter().map(|(o, _)| o.clone()).collect();
    let cycled = matches!(model.vertex_layout(), VertexLayout::AxisCycled);
    let frame = (model.version == 0).then_some(model.transform).flatten();
    let object_space = |p: Vec3| {
        let raw = if cycled { Vec3::new(-p.y, p.x, p.z) } else { p };
        frame.map_or(raw, |m| {
            Vec3::new(
                raw.x * m[0][0] + raw.y * m[1][0] + raw.z * m[2][0] + m[3][0],
                raw.x * m[0][1] + raw.y * m[1][1] + raw.z * m[2][1] + m[3][1],
                raw.x * m[0][2] + raw.y * m[1][2] + raw.z * m[2][2] + m[3][2],
            )
        })
    };
    for (i, face) in model.indices.iter().enumerate() {
        let slot = model.face_materials.get(i).copied().unwrap_or(0) as usize;
        let over = find_override(&model.materials, slot, &plain);
        if over.is_some_and(TextureOverride::is_variable_hidden) {
            continue;
        }
        let text = over
            .and_then(|o| plain.iter().position(|p| std::ptr::eq(p, o)))
            .is_some_and(|i| overrides[i].1);
        let diffuse_alpha = model.materials.get(slot).map_or(1.0, |m| m.diffuse[3]);
        // `[matl_alpha] 0` is opaque (sco.md); text textures cover only their glyphs.
        let alpha =
            over.and_then(|o| o.alpha).is_some_and(|a| a != 0) || diffuse_alpha < 0.999 || text;
        let mut v = [Vec3::ZERO; 3];
        let mut ok = true;
        for (k, &idx) in face.iter().enumerate() {
            match model.vertices.get(idx as usize) {
                Some(vx) => v[k] = object_space(vx.position),
                None => ok = false,
            }
        }
        if ok {
            tris.push(ExpectedTri {
                v,
                flags: TriFlags {
                    base,
                    alpha,
                    text,
                    two_sided: false,
                },
                mesh,
            });
        }
    }
}

/// Two crossed vertical quads of `height` × `width`, standing on the origin (alpha-tested).
pub(super) fn tree_billboard(height: f32, width: f32) -> Vec<ExpectedTri> {
    let hw = width * 0.5;
    let quad = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| {
        let flags = TriFlags {
            base: true,
            alpha: true,
            text: false,
            two_sided: true,
        };
        [
            ExpectedTri {
                v: [a, b, c],
                flags,
                mesh: 0,
            },
            ExpectedTri {
                v: [a, c, d],
                flags,
                mesh: 0,
            },
        ]
    };
    let mut out = Vec::new();
    out.extend(quad(
        Vec3::new(-hw, 0.0, 0.0),
        Vec3::new(hw, 0.0, 0.0),
        Vec3::new(hw, height, 0.0),
        Vec3::new(-hw, height, 0.0),
    ));
    out.extend(quad(
        Vec3::new(0.0, 0.0, -hw),
        Vec3::new(0.0, 0.0, hw),
        Vec3::new(0.0, height, hw),
        Vec3::new(0.0, height, -hw),
    ));
    out
}
