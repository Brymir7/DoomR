pub mod config {
    pub const SCREEN_WIDTH: u32 = 1920;
    pub const SCREEN_HEIGHT: u32 = 1080;
    pub const WORLD_WIDTH: u32 = 64;
    pub const WORLD_HEIGHT: u32 = 64;
    pub const PHYSICS_FRAME_TIME: f32 = 1.0 / 60.0;
    pub const TILE_SIZE_X_PIXEL: u32 = SCREEN_WIDTH / WORLD_WIDTH;
    pub const TILE_SIZE_Y_PIXEL: u32 =  SCREEN_HEIGHT / WORLD_HEIGHT ;
}