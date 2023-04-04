#version 450
precision mediump float;

uniform layout(binding=0) sampler2D read_u;
uniform layout(binding=1) sampler2D read_v;

in vec4 vert;
out vec4 out_color;
void main() {
    const float k = 10.;
    float u = texture(read_u, vert.xy).x / k;
    float v = texture(read_v, vert.xy).x / k;

    vec3 color = mix(
        vec3(0.1, 0.8, 0.9),
        vec3(0.8, 1., 0.1),
        length(vec2(u, v))
    );

    color *= float(fract(vert.w / 10.) < 0.1);

    out_color = vec4(color, 1.);
}
