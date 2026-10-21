struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) next_uv: vec2<f32>,
    @location(2) frame_mix: f32,
    @location(3) color: vec4<f32>,
};

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@vertex
fn vertex_main(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) next_uv: vec2<f32>,
    @location(3) frame_mix: f32,
    @location(4) color: vec4<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = uv;
    output.next_uv = next_uv;
    output.frame_mix = frame_mix;
    output.color = color;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let first = textureSample(atlas, atlas_sampler, input.uv);
    var sample = first;
    if input.frame_mix > 0.0 {
        let next = textureSample(atlas, atlas_sampler, input.next_uv);
        let alpha = mix(first.a, next.a, input.frame_mix);
        let premultiplied = mix(first.rgb * first.a, next.rgb * next.a, input.frame_mix);
        var rgb = vec3<f32>(0.0);
        if alpha > 0.0 {
            rgb = premultiplied / alpha;
        }
        sample = vec4<f32>(rgb, alpha);
    }
    return sample * input.color;
}
