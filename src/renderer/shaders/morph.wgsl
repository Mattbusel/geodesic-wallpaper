// Surface morphing vertex + fragment shader.
//
// Blends between two surface meshes (surface A and surface B) using the
// `morph_t` uniform.  Both meshes are uploaded as separate vertex buffers
// bound at locations 0 (surface A) and 1 (surface B).
//
// Set `morph_t = 0.0` for surface A, `1.0` for surface B, and any value
// in between for a smooth blend.  The host should drive this with the eased
// blend parameter from `SurfaceMorph::blend_t()`.

struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    /// Blend parameter: 0 = surface A, 1 = surface B.
    morph_t: f32,
    time: f32,
    _pad: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    /// Position on surface A.
    @location(0) pos_a: vec3<f32>,
    /// Position on surface B.
    @location(1) pos_b: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) morph_t: f32,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // GPU-side linear interpolation between the two surface meshes.
    let blended = mix(in.pos_a, in.pos_b, uniforms.morph_t);

    out.clip_position = uniforms.view_proj * vec4<f32>(blended, 1.0);
    out.world_pos = blended;
    out.morph_t = uniforms.morph_t;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Screen-space normals from world position.
    let edge1 = dpdx(in.world_pos);
    let edge2 = dpdy(in.world_pos);
    let normal = normalize(cross(edge1, edge2));

    let light = normalize(uniforms.light_dir.xyz);
    let diffuse = max(dot(normal, light), 0.0);
    let ambient = 0.08;
    let lit = ambient + (1.0 - ambient) * diffuse;

    // Colour morphs from deep blue (surface A) to warm gold (surface B).
    let color_a = vec3<f32>(0.10, 0.15, 0.35);
    let color_b = vec3<f32>(0.35, 0.25, 0.08);
    let base_color = mix(color_a, color_b, in.morph_t);

    // Slight glow during the morph transition.
    let glow = 1.0 + 0.3 * sin(in.morph_t * 3.14159);
    let final_color = base_color * lit * glow;

    // Alpha fades slightly during transition for a ghosting effect.
    let alpha = mix(0.30, 0.22, sin(in.morph_t * 3.14159));

    return vec4<f32>(final_color, alpha);
}
