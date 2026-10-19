// Light-cone prototypes for the lighting benchmark (docs/src/architecture/lighting-cones.md).
//
// Mesh cone (default): additive, fresnel edge fade, axial falloff, optional depth fade.
// CONE_MARCH: the proxy cone is drawn back faces only; the fragment intersects the view ray with
// the analytic cone, clips the segment to the depth prepass (when present) and integrates the
// scatter with a few steps.
//
// AlphaMode::Add shares the premultiplied pipeline: the shader must output alpha 0 for additive blending.
// Cone geometry comes from the model matrix: the apex is the translation, the axis is local -Z,
// the length is the Z scale and the base radius is the X scale. The colour is the mesh tag
// (packed rgb8).
import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_functions::{get_world_from_local, get_local_from_world, get_tag},
    mesh_view_bindings::view,
    view_transformations::depth_ndc_to_view_z,
};
@if(DEPTH_PREPASS) import bevy_pbr::prepass::utils::prepass_depth;

// x = intensity (per metre for the march, scalar for the mesh), y = edge power (mesh) / unused,
// z = depth fade distance in metres, w = axial falloff exponent.
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: vec4<f32>;
// x = march steps, y = soft edge start radius (0..1), z/w unused.
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> params2: vec4<f32>;

fn tag_color(tag: u32) -> vec3<f32> {
    return vec3<f32>(f32(tag & 255u), f32((tag >> 8u) & 255u), f32((tag >> 16u) & 255u)) / 255.0;
}

fn ign(p: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(p, vec2<f32>(0.06711056, 0.00583715))));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
@if(BALLAST) {
    // Pure ALU ballast that never writes: makes the frame GPU-bound so frame-time deltas equal GPU deltas.
    var x = in.position.x * 0.001 + params2.z;
    for (var i = 0u; i < u32(params2.x); i += 1u) {
        x = sin(x * 1.37) + cos(x * 0.71) * 0.5 + x * 0.01;
    }
    if x > 1.0e30 {
        return vec4<f32>(1.0);
    }
    discard;
}
    let m = get_world_from_local(in.instance_index);
    let apex = m[3].xyz;
    let axis_raw = -m[2].xyz;
    let len = length(axis_raw);
    let axis = axis_raw / len;
    let tan_a = length(m[0].xyz) / len;
    let color = tag_color(get_tag(in.instance_index));
    let P = in.world_position.xyz;

@if(CONE_MARCH) {
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
    // inside the cone: z^2 - x^2 - y^2 >= 0, i.e. g(t) = -(qa t^2 + qb t + qc) >= 0
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
            // inside the cone: qa t^2 + qb t + qc <= 0, between the roots when qa > 0
            t0 = max(t0, lo);
            t1 = min(t1, hi);
        } else if t0 < lo {
            t1 = min(t1, lo);
        } else {
            t0 = max(t0, hi);
        }
    }
@if(DEPTH_PREPASS) {
    let scene_z = depth_ndc_to_view_z(prepass_depth(in.position, 0u));
    let fwd = -view.world_from_view[2].xyz;
    let t_scene = (-scene_z) / max(dot(d, fwd), 1e-4);
    t1 = min(t1, t_scene);
}
    if t1 <= t0 { discard; }

    if params2.w > 0.5 && params2.w < 1.5 { return vec4<f32>(color * 0.3, 0.0); }
    if params2.w > 1.5 && params2.w < 2.5 { return vec4<f32>(vec3<f32>((t1 - t0) * 0.02), 0.0); }
    let steps = max(u32(params2.x), 1u);
    let dt = (t1 - t0) / f32(steps);
    let jit = ign(in.position.xy);
    var acc = 0.0;
    var dr = 10.0;
    var dax = 0.0;
    var dss = 0.0;
    for (var i = 0u; i < steps; i += 1u) {
        let t = t0 + (f32(i) + jit) * dt;
        let p = ol + dl * t;
        let s = -p.z;
        let rad = length(p.xy) / max(s, 1e-3);
        let radial = 1.0 - smoothstep(params2.y, 1.0, rad);
        let axial = pow(max(1.0 - s, 0.0), params.w);
        acc += radial * axial * smoothstep(0.0, 0.05, s);
        dr = min(dr, rad);
        dax = axial;
        dss = s;
    }
    let near = smoothstep(0.0, 1.5, t0 + 0.5 * (t1 - t0));
    if params2.w > 2.5 && params2.w < 4.5 { if params2.w < 3.5 { return vec4<f32>(dr, dax, dss, 0.0); }
        return vec4<f32>(acc * 0.1, dt * 0.2, near, 0.0); }
    if params2.w > 4.5 && params2.w < 5.5 { return vec4<f32>(params.x * 0.1, params.w * 0.1, acc * dt * 0.2, 0.0); }
    if params2.w > 5.5 && params2.w < 6.5 { return vec4<f32>(vec3<f32>(acc * dt * 3.0 * near), 0.0); }
    if params2.w > 6.5 { return vec4<f32>(color * (acc * dt * 3.0), 0.0); }
    return vec4<f32>(color * (acc * dt * params.x * near), 0.0);
} @else {
    let V = normalize(view.world_position - P);
    let N = normalize(in.world_normal);
    var a = pow(abs(dot(N, V)), params.y);
    let s = clamp(dot(P - apex, axis) / len, 0.0, 1.0);
    a *= pow(1.0 - s, params.w) * smoothstep(0.0, 0.05, s);
@if(DEPTH_PREPASS) {
    let scene_z = depth_ndc_to_view_z(prepass_depth(in.position, 0u));
    let frag_z = depth_ndc_to_view_z(in.position.z);
    a *= saturate((frag_z - scene_z) / max(params.z, 1e-3));
}
    a *= smoothstep(0.0, 1.5, length(view.world_position - P));
    return vec4<f32>(color * (a * params.x), 0.0);
}
}
