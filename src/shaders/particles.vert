#version 450

uniform layout(binding=0) sampler2D read_u;
uniform layout(binding=1) sampler2D read_v;
uniform layout(binding=1, r32f) image2D write_img;
layout(std430, binding=4) buffer Particles {
    vec2 particles[];
};
uniform vec2 screen_size;

out vec2 vert;
out float f_idx;

void main() {
    int idx = gl_VertexID;
    f_idx = float(idx);// / float(particles.length());

    vec2 size = vec2(imageSize(write_img));
    vert = particles[idx] / size;

    vec2 sp = vert;
        float u = texture(read_u, vert + vec2(0., 1)/size).x;
        float v = texture(read_v, vert + vec2(1, 0.)/size).x;
        vec2 uv = vec2(u, v);

        sp -= uv/size.x;

    sp.x *= min(screen_size.x, screen_size.y)/screen_size.x;
    sp.x *= size.x/size.y;
    sp = sp * 2. - 1.;

    gl_Position = vec4(sp, 0.0, 1.0);
    //gl_PointSize = 1.;
}
