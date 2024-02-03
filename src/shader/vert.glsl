#version 330

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 inUV;

out vec2 uv;

const vec2 verts[3] = vec2[3](vec2(0.5, 1.0), vec2(0.0, 0.0), vec2(1.0, 0.0));

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    uv = inUV;
}
