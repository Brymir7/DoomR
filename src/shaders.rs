pub mod shaders {
    pub const DEFAULT_VERTEX_SHADER: &'static str =
"#version 100
precision lowp float;

attribute vec3 position;
attribute vec2 texcoord;

varying vec2 uv;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}
";
    pub const FLOOR_FRAGMENT_SHADER: &'static str =
"#version 330 core

uniform vec2 u_player_pos;
uniform vec2 u_left_ray_dir;
uniform vec2 u_right_ray_dir;
uniform float u_half_screen_height;
uniform sampler2D u_floor_texture;
uniform float u_screen_width;
uniform float u_screen_height;
uniform float is_ceiling;
out vec4 FragColor;

void main()
{
    float row = gl_FragCoord.y;
    float col = gl_FragCoord.x;
    float row_distance = (u_half_screen_height / (row - u_half_screen_height + 0.01)) * is_ceiling;
    vec2 ray_dir = mix(u_left_ray_dir, u_right_ray_dir, col / u_screen_width);
    vec2 floor_pos = u_player_pos + ray_dir * row_distance;
    vec2 tex_coords = fract(floor_pos);
    vec4 tex_color = texture(u_floor_texture, tex_coords);
    float shade = clamp(1.0 - (row_distance / 5), 0.0, 1.0);
    FragColor = vec4(tex_color.rgb * shade, 1.0);
}
";
}