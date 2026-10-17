// Layered ground: layer 0 is the base, every further layer is blended over it by its
// per-tile mask. Layer textures repeat `repeats[i]` times across the tile and keep their own
// resolution and mip chain, so painted detail stays sharp up close.
import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
};

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var layer_tex: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var layer_smp: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var mask_tex: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var mask_smp: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(104) var<uniform> repeats_a: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(105) var<uniform> repeats_b: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(106) var<uniform> layer_count: vec4<u32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(107) var detail_tex: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(108) var detail_smp: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(109) var<uniform> detail_repeats_a: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var<uniform> detail_repeats_b: vec4<f32>;

@group(constants::MATERIAL_BIND_GROUP) @binding(111) var<uniform> wet: vec4<f32>;

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn repeats(i: u32) -> f32 {
    if i < 4u {
        return repeats_a[i];
    }
    return repeats_b[i - 4u];
}

fn detail_repeats(i: u32) -> f32 {
    if i < 4u {
        return detail_repeats_a[i];
    }
    return detail_repeats_b[i - 4u];
}

// A layer is its texture (`repeats` times across the tile) multiplied by its detail texture
// (`detail_repeats` times): the second and third numbers of `[groundtex]`.
fn layer_color(uv: vec2<f32>, i: u32) -> vec3<f32> {
    let t = textureSample(layer_tex, layer_smp, uv * repeats(i), i).rgb;
    let d = textureSample(detail_tex, detail_smp, uv * detail_repeats(i), i).rgb;
    return t * d;
}

fn ground_color(uv: vec2<f32>) -> vec4<f32> {
    var color = layer_color(uv, 0u);
    for (var i = 1u; i < layer_count.x; i = i + 1u) {
        let w = textureSample(mask_tex, mask_smp, uv, i).r;
        color = mix(color, layer_color(uv, i), w);
    }
    return vec4<f32>(color, 1.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = ground_color(in.uv);
    if wet.x > 0.0 {
        // Wet ground: darker, smoother; puddles (low-frequency noise above a threshold that
        // falls with the coverage) are darker still and nearly mirror-smooth.
        let p = in.world_position.xz;
        let n = 0.65 * value_noise(p * 0.22) + 0.35 * value_noise(p * 0.9 + 17.0);
        let puddle = smoothstep(1.0 - wet.y * wet.x, 1.0 - wet.y * wet.x + 0.08, n);
        let dark = mix(1.0, 0.7, wet.x) * mix(1.0, 0.75, puddle);
        pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * dark, pbr_input.material.base_color.a);
        let r = mix(pbr_input.material.perceptual_roughness, 0.35, wet.x);
        pbr_input.material.perceptual_roughness = mix(r, 0.04, puddle);
    }
    if wet.z > 0.0 {
        // Snow cover (weather `[snow]`): towards white-grey, rougher than wet ground.
        let snow_colour = vec3<f32>(0.92, 0.94, 0.97);
        pbr_input.material.base_color = vec4<f32>(mix(pbr_input.material.base_color.rgb, snow_colour, wet.z), pbr_input.material.base_color.a);
        pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.85, wet.z);
    }
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );

@if(PREPASS_PIPELINE)
    let out = deferred_output(in, pbr_input);
@if(!PREPASS_PIPELINE)
    var out: FragmentOutput;
@if(!PREPASS_PIPELINE)
    out.color = apply_pbr_lighting(pbr_input);
@if(!PREPASS_PIPELINE)
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
