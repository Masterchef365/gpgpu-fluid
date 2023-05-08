#version 450
precision mediump float;

uniform layout(binding=0) sampler2D read_u;
uniform layout(binding=1) sampler2D read_v;

in vec2 vert;
out vec4 out_color;
void main() {
    const float k = 10.;
    float u = texture(read_u, vert).x / k;
    float v = texture(read_v, vert).x / k;

    float t = clamp(length(vec2(u, v)) * 1.5, 0.3, 3.5);
    vec3 color = //mix(
        //vec3(0.15, 1., 0.11)/4.;
        //vec3(0.15, 0.41, 1.)/1.;
        //vec3(0.9, 0.3, 1.),
        //t
        vec3(1.)/3.;
    //) / 3.;

    out_color = vec4(color, 1.);
}
