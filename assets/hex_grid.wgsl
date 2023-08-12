// resources
// - 2d hex tiles shadertoy -- https://www.youtube.com/watch?v=VmrIDyYiJBA
// - computer graphics parametric lines & ray tracing
//   https://www.youtube.com/watch?v=RHRVBVSiy58&list=PLxGzv4uunL64DRA5DXKuUSJ0hIEbBU7w8&index=12
// - unprojections explained -- https://www.derschmale.com/2014/09/28/unprojections-explained/
// - infinite grid vulkan -- https://asliceofrendering.com/scene%20helper/2020/01/05/InfiniteGrid/

struct View {
    viewport: vec4<u32>,
    projection: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) near_point: vec3<f32>,
    @location(1) far_point: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> view: View;

fn unproject_point(pos: vec3<f32>) -> vec3<f32> {
    let point = view.view * view.inverse_projection * vec4(pos, 1.0);
    return point.xyz / point.w;
}

@vertex
fn vertex(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var grid_plane = array<vec3<f32>, 4>(
        vec3<f32>(-1., -1., 1.),
        vec3<f32>(-1., 1., 1.),
        vec3<f32>(1., -1., 1.),
        vec3<f32>(1., 1., 1.)
    );

    var out: VertexOutput;
    let pos = grid_plane[in_vertex_index];
    out.clip_position = vec4(pos, 1.0);
    out.near_point = unproject_point(pos);
    out.far_point = unproject_point(vec3<f32>(pos.xy, 0.00001));
    return out;
    // return vec4(unproject_point(grid_plane[in_vertex_index]), 1.0);
}


@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var col = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    let v = in.far_point - in.near_point;
    // calculate intersect with y = 0;
    let t = -in.near_point.y / v.y;
    if t <= 0.0 {
        return col;
    }
    let intersect = abs(in.near_point + t * v);
    let gv = fract(intersect);
    col.b = gv.x;
    col.r = gv.z;
    col.a = 1.0;
    // col.r = t * 10.0;
    //col.g = xz.y;

    // we need to calculate the xy coords where the near-far line intersects y=0
    // col.g = (in.near_point.y - 0.98) / 10.0;
    //col.g = (in.near_point.y - 0.95) * 10.0;
    return col;
}
