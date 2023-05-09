#version 450
precision mediump float;

uniform layout(binding=0) sampler2D read_u;
uniform layout(binding=1) sampler2D read_v;

in vec2 vert;
in float f_idx;

const float PI = 3.1415926;

out vec4 out_color;

vec3 hsv2rgb(vec3 c);

void main() {
    const float k = 10.;
    float u = texture(read_u, vert).x / k;
    float v = texture(read_v, vert).x / k;

    float t = clamp(length(vec2(u, v)) * 1.5, 0.3, 3.5);
    /*
    vec3 color = mix(
        vec3(1., 0.5, 0.01),
        vec3(0.9, 0.3, 1.),
        t
    ) / 3.;
    */

    vec3 color = hsv2rgb(vec3(fract((f_idx - 0.5) * 1.), 0.95, 1.));

    out_color = vec4(color, 1.);
}

vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1., 2. / 3., 1. / 3., 3.);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6. - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0., 1.), c.y);
}
