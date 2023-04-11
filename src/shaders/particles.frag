#version 450
precision mediump float;

uniform layout(binding=0) sampler2D read_u;
uniform layout(binding=1) sampler2D read_v;

in vec2 vert;
in float part_fg;

out vec4 out_color;
void main() {
    const float k = 10.;
    float u = texture(read_u, vert).x / k;
    float v = texture(read_v, vert).x / k;

    vec3 color = mix(
        vec3(0.5, 1., 0.01),
        vec3(1.),
        pow(u*u+v*v, 1./2.)
    );

    color *= float(part_fg);

    out_color = vec4(color, 1.);
}
