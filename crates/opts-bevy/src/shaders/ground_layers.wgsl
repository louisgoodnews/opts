// Layered ground: layer 0 is the base, every further layer is blended over it by its
// per-tile mask. Layer textures repeat `repeats[i]` times across the tile and keep their own
// resolution and mip chain, so painted detail stays sharp up close.
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var layer_tex: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var layer_smp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var mask_tex: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var mask_smp: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var<uniform> repeats_a: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var<uniform> repeats_b: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var<uniform> layer_count: vec4<u32>;

fn repeats(i: u32) -> f32 {
    if i < 4u {
        return repeats_a[i];
    }
    return repeats_b[i - 4u];
}

fn ground_color(uv: vec2<f32>) -> vec4<f32> {
    var color = textureSample(layer_tex, layer_smp, uv * repeats(0u), 0).rgb;
    for (var i = 1u; i < layer_count.x; i = i + 1u) {
        let w = textureSample(mask_tex, mask_smp, uv, i).r;
        let t = textureSample(layer_tex, layer_smp, uv * repeats(i), i).rgb;
        color = mix(color, t, w);
    }
    return vec4<f32>(color, 1.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = ground_color(in.uv);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif
    return out;
}
