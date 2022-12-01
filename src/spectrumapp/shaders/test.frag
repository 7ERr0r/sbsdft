#version 310 es

precision highp float;

struct VertexOutput {
    vec4 proj_position;
    vec4 color;
    vec2 tex_coord;
};

uniform highp sampler2D _group_0_binding_1;

smooth layout(location = 0) in vec4 _vs2fs_location0;
smooth layout(location = 1) in vec2 _vs2fs_location1;
layout(location = 0) out vec4 _fs2p_location0;

void main() {
    VertexOutput in1 = VertexOutput(gl_FragCoord, _vs2fs_location0, _vs2fs_location1);
    vec4 color2;
    vec4 _expr6 = texture(_group_0_binding_1, vec2(in1.tex_coord));
    color2 = (in1.color * _expr6);
    color2 = (color2 / vec4(((((((256.0 * 256.0) * 256.0) * 256.0) * 256.0) * 256.0) * 256.0)));
    _fs2p_location0 = color2;
    return;
}

