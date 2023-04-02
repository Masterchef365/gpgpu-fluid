#version 450

uniform layout(binding=0, rg32f) image2D read_img;
layout(std430, binding=1) buffer Particles {
    vec2 particles[];
};

void main() {
    vec2 vert = particles[gl_VertexID] / vec2(imageSize(read_img));
    vert = vert * 2. - 1.;
    gl_Position = vec4(vert, 0.0, 1.0);
    gl_PointSize = 5.;
}
