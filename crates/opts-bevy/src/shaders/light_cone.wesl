// Visible light cones (lanterns, vehicle headlights): docs/src/architecture/lighting-cones.md.
//
// A proxy cone mesh is drawn back faces only. The fragment intersects the view ray with the
// analytic cone, clips the segment to the depth prepass when the camera has one (the depth
// fade), and integrates the scatter along it with a few steps (8 near, 4 far). Additive: the
// output alpha must be 0 (AlphaMode::Add shares the premultiplied pipeline).
//
// Per-instance data comes from the model matrix and the mesh tag:
//   apex   = translation, axis = local -Z, length = |Z axis|, tan(half angle) = |X axis| / length
//   tag    = r(5 bits) | g(5) << 5 | b(5) << 10 | intensity(6) << 15 | inner edge(4) << 21
//            | ground height(7) << 25
// where the inner edge is the radius (0..1 of the outer radius) at which the soft falloff starts
// and the ground height is the apex height above the ground in 0.25 m steps (127 = unknown): the
// volume ends at that horizontal plane with a soft base, with or without the depth prepass.
//
// The output is `1 - exp(-scatter)` per cone and the pipeline blends with a screen blend
// (src + dst * (1 - src)), so overlapping cones cannot add up past white. Without the depth
// prepass (depth fade off) the cones also cross walls, so the scatter is halved.
import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_functions::{get_world_from_local, get_local_from_world, get_tag},
    mesh_view_bindings::view,
    view_transformations::depth_ndc_to_view_z,
};
@if(DEPTH_PREPASS) import bevy_pbr::prepass::utils::prepass_depth;

// x = scatter per metre of a fully lit cone centre, y = axial falloff exponent,
// z = distance (m) from the camera beyond which a cone uses the far step count,
// w = the most one cone adds to a pixel (0..1).
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: vec4<f32>;

const NEAR_STEPS: u32 = 8u;
const FAR_STEPS: u32 = 4u;

// Interleaved gradient noise: breaks up the banding of the few steps.
fn ign(p: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(p, vec2<f32>(0.06711056, 0.00583715))));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let m = get_world_from_local(in.instance_index);
    let tag = get_tag(in.instance_index);
    let color = vec3<f32>(
        f32(tag & 31u),
        f32((tag >> 5u) & 31u),
        f32((tag >> 10u) & 31u),
    ) / 31.0;
    let intensity = f32((tag >> 15u) & 63u) / 63.0;
    let inner = min(f32((tag >> 21u) & 15u) / 15.0, 0.95);
    let ground_q = (tag >> 25u) & 127u;
    if intensity <= 0.0 { discard; }

    let P = in.world_position.xyz;
    let O = view.world_position;
    let to_p = P - O;
    let t_exit = length(to_p);
    let d = to_p / t_exit;
    // Work in the unit cone's local space: apex at the origin, axis -Z, radius = -z, z in [-1, 0].
    let lfw = get_local_from_world(in.instance_index);
    let ol = (lfw * vec4<f32>(O, 1.0)).xyz;
    let dl = (lfw * vec4<f32>(d, 0.0)).xyz;
    var t0 = 0.0;
    var t1 = t_exit;
    // -1 <= z(t) <= 0 with z(t) = ol.z + dl.z t
    if abs(dl.z) < 1e-7 {
        if ol.z < -1.0 || ol.z > 0.0 { discard; }
    } else {
        let ta = (-1.0 - ol.z) / dl.z;
        let tb = (0.0 - ol.z) / dl.z;
        t0 = max(t0, min(ta, tb));
        t1 = min(t1, max(ta, tb));
    }
    // inside the cone: z^2 - x^2 - y^2 >= 0, i.e. qa t^2 + qb t + qc <= 0
    let qa = dl.x * dl.x + dl.y * dl.y - dl.z * dl.z;
    let qb = 2.0 * (ol.x * dl.x + ol.y * dl.y - ol.z * dl.z);
    let qc = ol.x * ol.x + ol.y * ol.y - ol.z * ol.z;
    let disc = qb * qb - 4.0 * qa * qc;
    if abs(qa) < 1e-9 {
        if abs(qb) < 1e-9 {
            if qc > 0.0 { discard; }
        } else {
            let r = -qc / qb;
            if qb > 0.0 { t1 = min(t1, r); } else { t0 = max(t0, r); }
        }
    } else if disc < 0.0 {
        if qa > 0.0 { discard; }
    } else {
        let sq = sqrt(disc);
        let r0 = (-qb - sq) / (2.0 * qa);
        let r1 = (-qb + sq) / (2.0 * qa);
        let lo = min(r0, r1);
        let hi = max(r0, r1);
        if qa > 0.0 {
            // inside the cone: between the roots
            t0 = max(t0, lo);
            t1 = min(t1, hi);
        } else if t0 < lo {
            t1 = min(t1, lo);
        } else {
            t0 = max(t0, hi);
        }
    }
@if(DEPTH_PREPASS) {
    // The depth fade: the volume ends where the scene geometry is.
    let scene_z = depth_ndc_to_view_z(prepass_depth(in.position, 0u));
    let fwd = -view.world_from_view[2].xyz;
    let t_scene = (-scene_z) / max(dot(d, fwd), 1e-4);
    t1 = min(t1, t_scene);
}
    let apex = m[3].xyz;
    // The ground plane under the lamp: the volume never extends below it.
    var ground_y = -1.0e30;
    if ground_q != 127u {
        ground_y = apex.y - f32(ground_q) * 0.25;
        let oy = O.y - ground_y;
        if abs(d.y) > 1e-6 {
            let t_ground = -oy / d.y;
            if oy >= 0.0 {
                if d.y < 0.0 { t1 = min(t1, t_ground); }
            } else if d.y > 0.0 {
                t0 = max(t0, t_ground);
            } else {
                discard;
            }
        } else if oy < 0.0 {
            discard;
        }
    }
    if t1 <= t0 { discard; }

    let far = distance(O, apex) > params.z;
    let steps = select(NEAR_STEPS, FAR_STEPS, far);
    let dt = (t1 - t0) / f32(steps);
    let jit = ign(in.position.xy);
    var acc = 0.0;
    for (var i = 0u; i < steps; i += 1u) {
        let t = t0 + (f32(i) + jit) * dt;
        let p = ol + dl * t;
        let s = -p.z;
        let rad = length(p.xy) / max(s, 1e-3);
        let radial = 1.0 - smoothstep(inner, 1.0, rad);
        let axial = pow(max(1.0 - s, 0.0), params.y);
        // A soft base: fade over the last 0.8 m above the ground plane.
        let above = O.y + d.y * t - ground_y;
        acc += radial * axial * smoothstep(0.0, 0.05, s) * smoothstep(0.0, 0.8, above);
    }
    // Fade out a cone the camera is almost inside the apex of (no hard pop at the lamp).
    let near = smoothstep(0.0, 1.5, t0 + 0.5 * (t1 - t0));
    var density = params.x;
@if(!DEPTH_PREPASS) {
    density = density * 0.5;
}
    let scatter = acc * dt * density * intensity * near;
    return vec4<f32>(color * (params.w * (1.0 - exp(-scatter))), 0.0);
}
