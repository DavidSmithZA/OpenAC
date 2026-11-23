// Glass behind the modern interface: the frame drawn so far, blurred by
// halving it a few times and growing it back (the dual filter), each step
// a handful of filtered taps around the pixel. Opaque out.

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One triangle over the whole target.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: VertexOut;
    out.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn tap(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(source, source_sampler, uv, 0.0).rgb;
}

// Halving: the centre four times and the four diagonals a texel out.
@fragment
fn fs_down(v: VertexOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(source));
    var sum = tap(v.uv) * 4.0;
    sum += tap(v.uv + vec2<f32>(-texel.x, -texel.y));
    sum += tap(v.uv + vec2<f32>(texel.x, -texel.y));
    sum += tap(v.uv + vec2<f32>(-texel.x, texel.y));
    sum += tap(v.uv + vec2<f32>(texel.x, texel.y));
    return vec4<f32>(sum / 8.0, 1.0);
}

// Doubling: four taps two texels out along the axes, four diagonals one
// texel out weighted twice.
@fragment
fn fs_up(v: VertexOut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(source));
    var sum = tap(v.uv + vec2<f32>(-2.0 * texel.x, 0.0));
    sum += tap(v.uv + vec2<f32>(2.0 * texel.x, 0.0));
    sum += tap(v.uv + vec2<f32>(0.0, -2.0 * texel.y));
    sum += tap(v.uv + vec2<f32>(0.0, 2.0 * texel.y));
    sum += tap(v.uv + vec2<f32>(-texel.x, -texel.y)) * 2.0;
    sum += tap(v.uv + vec2<f32>(texel.x, -texel.y)) * 2.0;
    sum += tap(v.uv + vec2<f32>(-texel.x, texel.y)) * 2.0;
    sum += tap(v.uv + vec2<f32>(texel.x, texel.y)) * 2.0;
    return vec4<f32>(sum / 12.0, 1.0);
}
