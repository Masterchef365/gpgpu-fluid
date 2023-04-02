#version 450

uniform layout(binding=1, r32f) image2D write_img;
layout(std430, binding=4) buffer Particles {
    vec2 particles[];
};
uniform vec2 screen_size;

void main() {
    vec2 vert = particles[gl_VertexID] / vec2(imageSize(write_img));
    vert.x *= min(screen_size.x, screen_size.y)/screen_size.x;
    vert = vert * 2. - 1.;

    gl_Position = vec4(vert, 0.0, 1.0);
    //gl_PointSize = 1.;
}
