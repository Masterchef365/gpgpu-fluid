#version 450
uniform layout(binding=1, rg32f) image2D write_img;
layout(std430, binding=0) buffer Particles {
    vec2 particles[];
};

void main() {
    vec2 vert = particles[gl_VertexID] / vec2(imageSize(write_img));
    vert = vert * 2. - 1.;
    gl_Position = vec4(vert, 0.0, 1.0);
    gl_PointSize = 5.;
}
