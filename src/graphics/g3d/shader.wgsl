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
fn vertex_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var TRI_VERTICES = array<vec4<f32>, 3>(
        vec4(0.0, 0.0, 0.0, 1.0),
        vec4(0.0, 1.0, 0.0, 1.0),
        vec4(1.0, 1.0, 0.0, 1.0),
    );
    let position = TRI_VERTICES[vertex_index];
    #ifdef COLOR
    let color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    #endif
    #ifdef NORMAL
    let normal = vec3<f32>(0.0, 0.0, -1.0);
    #endif
    return VertexOut(
        position,
        #ifdef COLOR
        vec4<f32>(1.0, 0.0, 0.0, 1.0),
        #endif
        #ifdef NORMAL
        vec3<f32>(0.0, 0.0, -1.0),
        #endif
        #ifdef UV
        vec2<f32>(0.0, 0.0),
        #endif
    );
}

@fragment
fn fragment_main(in: FragmentIn) -> @location(0) vec4<f32> {
  return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}