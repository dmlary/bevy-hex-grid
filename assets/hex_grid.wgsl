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
    @location(2) cursor_hex: vec2<f32>,     // cursor hex coordinates
    @location(3) cursor_hex_edge_dist: f32, // distance of cursor from hex edge
    @location(4) cursor_hex_angle: f32,     // polar coord of cursor in hex
                                            // round
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

    let cursor = hex_coords(view.cursor_pos);
    out.cursor_hex = cursor.coords;
    out.cursor_hex_edge_dist = cursor.edge_dist;
    out.cursor_hex_angle = cursor.angle - cursor.angle % (radians(360.0 / 6.0));
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
    angle: f32,
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
    out.edge_dist = 0.5 - hex_dist(center);
    out.angle = atan2(center.x, center.y) + radians(180.0);
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
    if t <= 0.0 {
        return out;
    }

    let intersect = in.near_point + t * v;

    // calculate the depth from the intersect point
    let clipped = view.projection * view.inverse_view * vec4(intersect, 1.0);
    out.depth = clipped.z / clipped.w;


    let hex = hex_coords(intersect.xz);

    var step = 0.04;
    out.color = vec4(0.6);

    if distance(hex.coords, in.cursor_hex) < 0.1 {
        out.color = vec4(1.0, 0.8, 0.2, 1.0);
        step = 0.23;
        if in.cursor_hex_edge_dist < 0.15 {
            let h = hex.angle - hex.angle % radians(360.0 / 6.0);
            if h == in.cursor_hex_angle {
                out.color = vec4(0.2, 0.8, 1.0, 1.0);
                step = 0.23;
            }
        }
    }

    // lazy anti-alias using smoothstep
    out.color *= (1.0 - smoothstep(0.0, step, hex.edge_dist));

    return out;
}
