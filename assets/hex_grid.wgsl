struct View {
    viewport: vec4<u32>,
    projection: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> view: View;

@vertex
fn vertex(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    var grid_plane = array<vec4<f32>, 4>(
        vec4<f32>(-1., -1., 1., 1.),
        vec4<f32>(-1., 1., 1., 1.),
        vec4<f32>(1., -1., 1., 1.),
        vec4<f32>(1., 1., 1., 1.)
    );
    return grid_plane[in_vertex_index];
}

@fragment
fn fragment(@builtin(position) in: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = in.xy / vec2<f32>(view.viewport.zw);

    return vec4<f32>(uv.x, 0.0, uv.y, 1.0);
}
