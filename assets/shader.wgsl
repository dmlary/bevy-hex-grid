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
    return vec4<f32>(0.1, 0.2, 0.3, 1.0);
}
