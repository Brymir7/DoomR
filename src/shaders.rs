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
    float shade = clamp(1.0 - (row_distance / 15), 0.0, 1.0);
    FragColor = vec4(tex_color.rgb * shade, 1.0);
}
";
    pub const CAMERA_SHAKE_VERTEX_SHADER: &'static str =
        "#version 100
precision lowp float;

attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
uniform vec2 screen_size;
uniform vec2 shake_offset;
varying vec2 uv;
varying vec4 color;

void main() {
    vec4 modelPosition = vec4(position.xy + shake_offset, position.z, 1.0);
    modelPosition.xy /= screen_size / 2.0;
    modelPosition.xy -= 1.0;
    modelPosition.y *= -1.0;

    gl_Position = modelPosition;

    uv = texcoord;
    color = color0 / 255.0;
    }   
";
    pub const DEFAULT_FRAGMENT_SHADER: &'static str =
        "#version 100
precision lowp float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}
";
pub const ENEMY_DEFAULT_VERTEX_SHADER: &'static str =
"#version 100
precision lowp float;

attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

uniform vec2 screen_size;
varying vec2 uv;
varying vec4 color;

void main() {
    vec4 modelPosition = vec4(position, 1);

    modelPosition.xy /= screen_size / 2.0;
    modelPosition.xy -= 1.0;
    modelPosition.y *= -1.0;

    gl_Position = modelPosition;
    uv = texcoord;
    color = color0 / 255.0;
}
";
pub const ENEMY_DEFAULT_FRAGMENT_SHADER: &'static str =
"#version 100
precision lowp float;
uniform float u_relative_health;
uniform sampler2D Texture;

varying vec2 uv;
varying vec4 color;

void main() {
    vec4 textureColor = texture2D(Texture, uv);
    float redIntensity = (1.0 - u_relative_health) * 0.5; 
    float chance = (1.0 - u_relative_health) * 0.5; 
    
    vec4 redColor = vec4(1.0, 0.0, 0.0, 1.0);

    float randomValue = fract(sin(dot(uv.xy + gl_FragCoord.xy, vec2(12.9898, 78.233))) * 43758.5453);

    if (randomValue < chance) {
        gl_FragColor = vec4(mix(textureColor.rgb, redColor.rgb, redIntensity), textureColor.a) * color;
    } else {
        gl_FragColor = vec4(textureColor.rgb, textureColor.a) * color;
    }
}
";
}

