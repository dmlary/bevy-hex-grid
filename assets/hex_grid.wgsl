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


fn hex_dist(in: vec2<f32>) -> f32 {
    let uv = abs(in);
    var c = dot(uv, normalize(vec2<f32>(1.0, sqrt(3.0))));
    c = max(c, uv.x);
    return c;
}

@fragment
fn fragment(@builtin(position) in: vec4<f32>) -> @location(0) vec4<f32> {
    let res = vec2<f32>(view.viewport.zw);
    var uv = (in.xy - (res * 0.5)) / res.y;

    var col = vec3<f32>(0.0);

    uv *= 5.0;
    var a = fract(uv) - 0.5;
    var b = fract(uv - 0.5) - 0.5;

    var gv: vec2<f32>;
    if length(a) < length(b) {
        gv = a;
    } else {
        gv = b;
    }

    col.r = gv.x;
    col.g = gv.y;

    return vec4<f32>(col.x, col.y, 0.0, 1.0);
}
