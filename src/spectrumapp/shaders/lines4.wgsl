//[[block]]
struct Globals {
    view_proj: mat4x4<f32>,
};

@group(0)
@binding(0)
var<uniform> u_globals: Globals;





struct VertexOutput {
    @builtin(position) proj_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) in_position: vec2<i32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    var scale: f32 = 1.0/8.0;
    var ipos: vec2<f32> = scale * vec2<f32>(f32(in_position.x), f32(in_position.y));
    //var ipos: vec2<f32> = scale * vec2<f32>(in_position);
    var pos: vec4<f32> = vec4<f32>(ipos.x, ipos.y, 0.0, 1.0);
    out.proj_position = u_globals.view_proj * pos;
    //out.proj_position = pos;

    out.color = color;
    

    
    //out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    return out;
}















// fragment shader




@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec4<f32> = vec4<f32>(in.color);

    // webglcolorfix
    return color;// * vec4<f32>(1.0, 1.0, 1.0, 1.0);
    //return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}