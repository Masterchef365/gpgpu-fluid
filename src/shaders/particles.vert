#version 450
const vec2 verts[3] = vec2[3](
    vec2(0.5f, 1.0f),
    vec2(0.0f, 0.0f),
    vec2(1.0f, 0.0f)
);

out vec2 vert;

void main() {
    vert = verts[gl_VertexID];
    gl_Position = vec4(vert - 0.5, 0.0, 1.0);
}
