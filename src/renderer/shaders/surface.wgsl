struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
    time: f32,
    _pad: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(in.position, 1.0);
    out.world_pos = in.position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Compute screen-space normal from world position derivatives.
    let edge1 = dpdx(in.world_pos);
    let edge2 = dpdy(in.world_pos);
    let normal = normalize(cross(edge1, edge2));

    let light = normalize(uniforms.light_dir.xyz);
    let diffuse = max(dot(normal, light), 0.0);
    let lit = 0.1 + 0.9 * diffuse;

    return vec4<f32>(0.15 * lit, 0.18 * lit, 0.28 * lit, 0.25);
}
