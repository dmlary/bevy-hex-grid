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
    inverse_view: mat4x4<f32>,
    cursor_pos: vec2<f32>,
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
}

fn mod_euclid(p: vec2<f32>, m: vec2<f32>) -> vec2<f32> {
    var r = p % m;
    if r.x < 0.0 {
        r.x += m.x;
    }
    if r.y < 0.0 {
        r.y += m.y;
    }
    return r;
}

struct HexCoords {
    coords: vec2<f32>,
    edge_dist: f32,
};

fn hex_dist(pos: vec2<f32>) -> f32 {
    let p = abs(pos);
    let c = dot(p, normalize(vec2(1.0, sqrt(3.0))));
    return max(c, p.x);
}

fn hex_coords(uv: vec2<f32>) -> HexCoords {
    let dist = vec2<f32>(1.0, sqrt(3.0));
    let half_dist = dist * 0.5;

    // calculating the nearest hexagon center point.
    let a = mod_euclid(uv, dist) - half_dist;
    let b = mod_euclid(uv - half_dist, dist) - half_dist;

    var out: HexCoords;
    var center: vec2<f32>;
    if length(a) < length(b) {
        center = a;
    } else {
        center = b;
    };

    out.coords = uv - center;
    out.edge_dist = hex_dist(center);
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    // calculate intersect with y = 0;
    let v = in.far_point - in.near_point;
    let t = -in.near_point.y / v.y;
    let intersect = in.near_point + t * v;

    // calculate the depth from the intersect point
    let clipped = view.projection * view.inverse_view * vec4(intersect, 1.0);
    out.depth = clipped.z / clipped.w;

    if t <= 0.0 {
        return out;
    }
    let hex = hex_coords(intersect.xz);

    var step: f32;
    if distance(hex.coords, view.cursor_pos) <= 0.5 {
        out.color = vec4(1.0, 0.8, 0.2, 1.0);
        step = 0.33;
    } else {
        out.color = vec4(0.6);
        step = 0.46;
    }

    // lazy anti-alias using smoothstep
    out.color *= smoothstep(step, 0.5, hex.edge_dist);

    return out;
}
