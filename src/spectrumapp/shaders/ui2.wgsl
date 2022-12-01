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
    @location(1) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) in_position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    var ipos: vec2<f32> = vec2<f32>(in_position.x, in_position.y);
    var scale: f32 = 1.0;
    var pos: vec4<f32> = vec4<f32>(ipos.x * scale, ipos.y * scale, 0.0, 1.0);
    out.proj_position = u_globals.view_proj * pos; // XD
    //out.proj_position = pos;

    out.color = color;
    
    out.tex_coord = tex_coord;
    //out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    return out;
}















// fragment shader

@group(0)
@binding(1)
var r_color: texture_2d<f32>;
@group(0)
@binding(2)
var r_sampler: sampler;



@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var texcolor: vec4<f32> = textureSample(r_color, r_sampler, in.tex_coord);
    if (texcolor.a < 0.5) {
        discard;
    }
    var color: vec4<f32> = in.color * texcolor;



    // webgltexfix
    // color.b = 0.0;
    // color.g = 0.0;

    //// webglcolorfix
    return vec4<f32>(color.r, color.g, color.b, 1.0);
    //return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}