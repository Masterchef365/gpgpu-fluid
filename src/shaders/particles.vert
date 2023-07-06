#version 450

uniform layout(binding=0) sampler2D read_u;
uniform layout(binding=1) sampler2D read_v;
uniform layout(binding=1, r32f) image2D write_img;
layout(std430, binding=4) buffer Particles {
    vec2 particles[];
};
uniform vec2 screen_size;
uniform float u_time;

const float TAU = 6.2831853071;

out vec2 vert;
out float f_idx;

mat4 perspective(float fov_y_radians, float aspect_ratio, float z_near, float z_far) {
    float theta = 0.5 * fov_y_radians;
    float h = 1. / tan(theta);
    float w = h / aspect_ratio;
    float r = z_far / (z_far - z_near);
    return mat4(
        w, 0.0, 0.0, 0.0,
        0.0, h, 0.0, 0.0,
        0.0, 0.0, r, 1.0,
        0.0, 0.0, -r * z_near, 0.0
    );
}

void main() {
    int idx = gl_VertexID;
    f_idx = float(idx) / float(particles.length());

    vec2 size = vec2(imageSize(write_img));
    vert = particles[idx] / size;

    vec2 sp = vert;
        float u = texture(read_u, vert + vec2(0., 1)/size).x;
        float v = texture(read_v, vert + vec2(1, 0.)/size).x;
        vec2 uv = vec2(u, v);

        sp -= uv/size.x;

    //sp.x *= min(screen_size.x, screen_size.y)/screen_size.x;
    //sp.x *= size.x/size.y;
    //sp = sp * 2. - 1.;

    float a = sp.x * TAU + u_time/8.;
    float b = sp.y * TAU / 2.;
    vec3 pos = vec3(
        cos(a) * sin(b),
        -cos(b),
        sin(a) * sin(b)
    );
    pos *= 1. + float(idx)/float(particles.length());

    pos.z += 4.2;

    mat4 persp = perspective(1., size.x/size.y, 0.01, 1000.);

    gl_Position = persp * vec4(pos, 1.0);
    //gl_PointSize = 1.;
}
