#version 100

in vec2 pos;
uniform vec4 rect_color;
uniform vec2 screen_size;
uniform mat4 model;

varying vec4 color;
void main() {

    vec4 position = model * vec4(pos, 0.0, 1.0);
    position.x /= (screen_size.x / screen_size.y);
    position.xy = position.xy / screen_size * 2.0 - 1.0;
    position.y *= -1.0;
    gl_Position = position;
    color = rect_color;
}
    
    