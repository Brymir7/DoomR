use macroquad::{
    color::Color,
    prelude::ImageFormat,
    texture::{Image, Texture2D},
};

pub fn is_white(color: Color) -> bool {
    color.r == 1.0 && color.g == 1.0 && color.b == 1.0
}

pub fn convert_white_to_transparent(image: &mut Image) {
    for pixel in image.get_image_data_mut().iter_mut() {
        if is_white((*pixel).into()) {
            *pixel = Color::new(0.0, 0.0, 0.0, 0.0).into(); // Transparent color
        }
    }
}

pub fn load_and_convert_texture(data: &[u8], format: ImageFormat) -> Texture2D {
    let texture = Texture2D::from_file_with_format(data, Some(format));
    let mut texture_data = texture.get_texture_data();
    convert_white_to_transparent(&mut texture_data);
    texture.update(&texture_data);
    texture
}