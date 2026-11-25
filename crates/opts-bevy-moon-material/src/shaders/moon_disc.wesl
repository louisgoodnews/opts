// The moon disc: the moon photograph with the terminator of its phase, drawn additively on the
// sky. The photograph has no alpha and a light background, so the disc is cut out with a circle.
//
// params.x  phase angle in radians (0 full, pi new)
// params.y  direction of the bright limb in the disc plane, radians counter-clockwise from the
//           disc's right
// params.z  brightness (daylight and weather fade)
// params.w  earthshine, the brightness of the unlit part (0..1)
import bevy_pbr::render::forward_io::VertexOutput;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var moon_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var moon_smp: sampler;

// Radius of the disc inside the quad, in quad half-widths.
const DISC_RADIUS: f32 = 0.94;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // The quad is oriented with its front towards the viewer: uv.x runs right, uv.y down.
    let p = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0) / DISC_RADIUS;
    let r2 = dot(p, p);
    // Sampled before any discard so the sample is in uniform control flow.
    let tex = textureSample(moon_tex, moon_smp, in.uv).rgb;
    if r2 > 1.0 {
        discard;
    }
    let n = vec3<f32>(p, sqrt(1.0 - r2));
    let alpha = params.x;
    let chi = params.y;
    let l = vec3<f32>(sin(alpha) * cos(chi), sin(alpha) * sin(chi), cos(alpha));
    let lit = smoothstep(-0.03, 0.03, dot(n, l));
    // A soft rim keeps the edge from aliasing against the sky.
    let rim = 1.0 - smoothstep(0.96, 1.0, sqrt(r2));
    let shade = params.w + (1.0 - params.w) * lit;
    // Premultiplied pipeline: alpha 0 adds the colour to the sky.
    return vec4<f32>(tex * shade * params.z * rim, 0.0);
}
