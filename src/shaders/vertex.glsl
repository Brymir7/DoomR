#version 100

in vec2 pos;
in vec4 color0;
uniform vec2 screen_size;
uniform mat4 model;

varying vec4 color;
void main() {
    vec4 position = model*vec4(pos, 0.0, 1.0);
    position.xy = position.xy * 0.25 / screen_size * 2.0 + 0.5;
    gl_Position = position ;
    color = color0;
}