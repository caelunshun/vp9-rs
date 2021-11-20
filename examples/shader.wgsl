struct VertexOutput {
    [[location(0)]] texcoord: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

// Vertex shader to generate a fullscreen quad without
// vertex buffers. See:
// https://www.saschawillems.de/blog/2016/08/13/vulkan-tutorial-on-rendering-a-fullscreen-quad-without-buffers/.
[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    out.texcoord = vec2<f32>(f32((i32(vertex_index) << u32(1)) & 2), f32(i32(vertex_index) & 2));
    out.position = vec4<f32>(out.texcoord.x * 2.0 - 1.0, -(out.texcoord.y * 2.0 - 1.0), 0.0, 1.0);

    return out;
}

[[group(0), binding(0)]] var sampler: sampler;
[[group(0), binding(1)]] var texture_y: texture_2d<f32>;
[[group(0), binding(2)]] var texture_u: texture_2d<f32>;
[[group(0), binding(3)]] var texture_v: texture_2d<f32>;

// Fragment shader to decode the YUV data and convert it to RGB.
[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var yuv: vec3<f32>;
    yuv.x = textureSample(texture_y, sampler, in.texcoord).r;
    yuv.y = textureSample(texture_u, sampler, in.texcoord).r;
    yuv.z = textureSample(texture_v, sampler, in.texcoord).r;
    
    yuv = yuv + vec3<f32>(-0.0627451017, -0.501960814, -0.501960814);
    
    var color: vec4<f32>;
    color.r = dot(yuv, vec3<f32>(1.164, 0.000, 1.596));
    color.g = dot(yuv, vec3<f32>(1.164, -0.391, -0.813));
    color.b = dot(yuv, vec3<f32>(1.164,  2.018,  0.000));
    color.a = 1.0;

    return color;
}