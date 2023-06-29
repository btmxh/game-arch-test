struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) orig_pos: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) radius: vec2<f32>,
    @location(3) pos_bounds: array<vec2<f32>, 2>,
}

struct Quad {
    pos_bounds: array<vec2<f32>, 2>,
    radius: vec2<f32>,
    tex_bounds: array<vec2<f32>, 2>,
    transform: mat3x3<f32>,
}

const MIX_TEX_COORDS = array<vec2<f32>>(
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 1.0)
);

var<push_constant> quad: Quad;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var ret: VertexOutput;

    const u = in_vertex_index % 2u32;
    const test = in_vertex_index < 2u32;
    const v = select(0, 1, test);

    const x = quad.pos_bounds[u].x;
    const y = quad.pos_bounds[v].y;
    ret.orig_pos = vec2(x, y);
    const pos = quad.transform * vec3(ret.orig_pos, 1.0);
    ret.position = vec4(pos.xy, 0.0, pos.z);
    ret.tex_coords = mix(quad.tex_bounds[0], quad.tex_bounds[1], MIX_TEX_COORDS[in_vertex_index]);
    ret.radius = quad.radius;
    ret.pos_bounds[0] = quad.pos_bounds[0] + quad.radius;
    ret.pos_bounds[1] = quad.pos_bounds[1] - quad.radius;
    return ret;
}

struct FragmentInput {
    @location(0) orig_pos: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) radius: vec2<f32>,
    @location(3) pos_bounds: array<vec2<f32>, 2>,
}

@group(0)
@binding(1)
var tex: texture_2d<f32>;

@group(0)
@binding(2)
var tex_sampler: sampler;

@fragment
fn fs_main(frag: FragmentInput) -> @location(0) vec4<f32> {
    const max_distance = 0.01;
    const offset = clamp(frag.orig_pos, frag.pos_bounds[0], frag.pos_bounds[1]) - frag.orig_pos;
    const normalized_offset = offset / frag.radius;
    const distance = length(normalized_offset);
    const alpha = 1.0 - smoothstep(1.0, 1.0 + max_distance, distance);

    var color = textureSample(tex, tex_sampler, frag.tex_coords);
    color.a *= alpha;
    return color;
}
