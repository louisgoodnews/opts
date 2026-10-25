// Trees move in the wind (M-0144, docs/src/architecture/vegetation-wind.md). The vertex stage of
// Bevy's mesh and prepass shaders with one addition: the vertex is bent along the wind. The foot
// stays; the displacement grows with the height fraction (1 - v, the billboard's texture runs from
// v = 1 at the ground to v = 0 at the top) times the height above the foot, so the crown swings and
// the trunk barely moves. Trees carry no skin and no morph targets.

@if(PREPASS_PIPELINE)
import bevy_pbr::prepass::io::{Vertex, VertexOutput, decompress_vertex};
@else
import bevy_pbr::render::forward_io::{Vertex, VertexOutput, decompress_vertex};

import bevy_pbr::render::{
    mesh_functions,
    view_transformations::position_world_to_clip,
};

// The shader globals (time): the prepass binds them as binding 1 of its view group, the main pass
// as binding 11 (`mesh_view_bindings`).
@if(PREPASS_PIPELINE)
import bevy_render::globals::Globals;
@if(!PREPASS_PIPELINE)
import bevy_pbr::render::mesh_view_bindings::globals;

@if(PREPASS_PIPELINE)
@group(0) @binding(1) var<uniform> globals: Globals;

// x, y: the direction the wind blows towards (Bevy x and z); z: crown displacement in metres for
// a 10 m tree (0 keeps the tree still); w: sway frequency in Hz.
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> wind: vec4<f32>;

// The bend of a vertex at `local` (metres above the foot in y) with texture height fraction `h`,
// for a tree whose foot is at `foot` (world), at time `t`.
fn bend(local: vec3<f32>, h: f32, foot: vec3<f32>, t: f32) -> vec3<f32> {
    if wind.z <= 0.0 {
        return vec3<f32>(0.0);
    }
    let phase = dot(foot.xz, vec2<f32>(0.131, 0.217));
    let w = 6.2831853 * wind.w;
    let along = sin(t * w + phase) * 0.6 + sin(t * w * 2.3 + phase * 1.7) * 0.3 + 0.4;
    let across = sin(t * w * 1.3 + phase * 2.9) * 0.25;
    let amount = wind.z * clamp(h, 0.0, 1.0) * max(local.y, 0.0) / 10.0;
    let dir = vec2<f32>(wind.x, wind.y);
    let side = vec2<f32>(-dir.y, dir.x);
    let off = (dir * along + side * across) * amount;
    return vec3<f32>(off.x, 0.0, off.y);
}

@vertex
fn vertex(vertex_in: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let foot = world_from_local[3].xyz;

@if(VERTEX_UVS_A)
    let h = 1.0 - vertex.uv.y;
@else
    let h = 0.0;

    var world = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    world = vec4<f32>(world.xyz + bend(vertex.position, h, foot, globals.time), world.w);
    out.world_position = world;
    out.position = position_world_to_clip(world.xyz);

@if(PREPASS_PIPELINE && UNCLIPPED_DEPTH_ORTHO_EMULATION) {
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
}

@if(VERTEX_UVS_A)
    out.uv = vertex.uv;
@if(VERTEX_UVS_B)
    out.uv_b = vertex.uv_b;

@if(PREPASS_PIPELINE && NORMAL_PREPASS_OR_DEFERRED_PREPASS && VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);
@if(!PREPASS_PIPELINE && VERTEX_NORMALS)
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex_in.instance_index);

@if(VERTEX_TANGENTS && (!PREPASS_PIPELINE || NORMAL_PREPASS_OR_DEFERRED_PREPASS))
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(world_from_local, vertex.tangent, vertex_in.instance_index);

@if(VERTEX_COLORS)
    out.color = vertex.color;

@if(PREPASS_PIPELINE && MOTION_VECTOR_PREPASS) {
    let prev_model = mesh_functions::get_previous_world_from_local(vertex_in.instance_index);
    let prev = mesh_functions::mesh_position_local_to_world(prev_model, vec4<f32>(vertex.position, 1.0));
    out.previous_world_position = vec4<f32>(prev.xyz + bend(vertex.position, h, prev_model[3].xyz, globals.time - globals.delta_time), prev.w);
}

@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_in.instance_index;

@if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(vertex_in.instance_index, world_from_local[3]);

    return out;
}
