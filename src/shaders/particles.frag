#version 450
precision mediump float;

in vec2 vert;
out vec4 color;
void main() {
    color = vec4(vert, 0.5, 1.0);
}
