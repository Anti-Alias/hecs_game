struct VertexIn {
    @location(0) position: vec3<f32>,
    #ifdef COLOR
    @location(1) color: vec4<f32>,
    #endif
    #ifdef NORMAL
    @location(2) normal: vec3<f32>,
    #endif
    #ifdef UV
    @location(3) uv: vec2<f32>,
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

@vertex
fn vertex_main(in: VertexIn) -> VertexOut {
    return VertexOut(
        vec4<f32>(in.position, 1.0),
        #ifdef COLOR
        in.color,
        #endif
        #ifdef NORMAL
        in.normal,
        #endif
        #ifdef UV
        in.uv,
        #endif
    );
}

@fragment
fn fragment_main(in: FragmentIn) -> @location(0) vec4<f32> {
  return in.color;
}