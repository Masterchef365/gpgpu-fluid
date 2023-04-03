#version 450

uniform layout(binding=1, r32f) image2D write_img;
layout(std430, binding=4) buffer Particles {
    vec2 particles[];
};
uniform vec2 screen_size;

out vec2 vert;

void main() {
    vert = particles[gl_VertexID] / vec2(imageSize(write_img));
    vec2 sp = vert;
    sp.x *= min(screen_size.x, screen_size.y)/screen_size.x;
    sp = sp * 2. - 1.;

    gl_Position = vec4(sp, 0.0, 1.0);
    //gl_PointSize = 1.;
}
