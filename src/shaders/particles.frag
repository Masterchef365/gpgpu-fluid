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

    vec3 color = mix(
        vec3(0.01, 0.5, 1.),
        vec3(1., 0.1, 0.01),
        length(vec2(u, v))
    ) / 10.;

    out_color = vec4(color, 1.);
}
