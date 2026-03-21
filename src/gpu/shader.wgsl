struct Globals {
    screen_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct VertexIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let sx = globals.screen_size.x;
    let sy = globals.screen_size.y;
    let ndc_x = (in.pos.x / sx) * 2.0 - 1.0;
    let ndc_y = 1.0 - (in.pos.y / sy) * 2.0;
    out.clip_pos = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, in.uv);
    return t * in.color;
}
