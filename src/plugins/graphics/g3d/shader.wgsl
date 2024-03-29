struct InstanceIn {
    @location(0) model_0: vec4<f32>,
    @location(1) model_1: vec4<f32>,
    @location(2) model_2: vec4<f32>,
    @location(3) model_3: vec4<f32>,
}

struct VertexIn {
    @location(4) position: vec3<f32>,
    #ifdef COLOR
    @location(5) color: vec4<f32>,
    #endif
    #ifdef NORMAL
    @location(6) normal: vec3<f32>,
    #endif
    #ifdef UV
    @location(7) uv: vec2<f32>,
    #endif
}

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    #ifdef COLOR
    @location(0) color: vec4<f32>,
    #endif
    #ifdef NORMAL
    @location(1) normal: vec3<f32>,
    #endif
    #ifdef UV
    @location(2) uv: vec2<f32>,
    #endif
}

struct FragmentIn {
    @builtin(position) position: vec4<f32>,
    #ifdef COLOR
    @location(0) color: vec4<f32>,
    #endif
    #ifdef NORMAL
    @location(1) normal: vec3<f32>,
    #endif
    #ifdef UV
    @location(2) uv: vec2<f32>,
    #endif
}

struct Uniform {
    base_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uni: Uniform;
#ifdef BASE_COLOR_TEX
@group(0) @binding(1)
var base_color_tex: texture_2d<f32>;
@group(0) @binding(2)
var base_color_sam: sampler;
#endif

@vertex
fn vertex_main(instance: InstanceIn, vert: VertexIn) -> VertexOut {
    let mvp = mat4x4<f32>(
        instance.model_0,
        instance.model_1,
        instance.model_2,
        instance.model_3,
    );
    return VertexOut(
        mvp * vec4<f32>(vert.position, 1.0),
        #ifdef COLOR
        vert.color,
        #endif
        #ifdef NORMAL
        vert.normal,
        #endif
        #ifdef UV
        vert.uv,
        #endif
    );
}

@fragment
fn fragment_main(in: FragmentIn) -> @location(0) vec4<f32> {
    var color = uni.base_color;

    // Base color texture
    #ifdef UV
    #ifdef BASE_COLOR_TEX
    color *= textureSample(base_color_tex, base_color_sam, in.uv);
    #endif
    #endif

    // Vertex colors
    #ifdef COLOR
    color *= in.color;
    #endif

    return color;
}