pub mod config {
    use std::f32::consts::PI;
    pub const WORLD_LAYOUT: [[u8; 15]; 9] = [
        [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
        [1,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
        [1,0,1,0,0,0,0,0,1,1,1,1,0,0,1],
        [1,0,0,0,0,0,0,0,0,0,0,1,0,0,1],
        [1,0,0,0,0,1,0,0,0,0,0,0,0,0,1],
        [1,0,1,0,0,1,0,0,0,0,1,0,1,0,1],
        [1,0,0,0,0,1,0,0,0,0,0,0,0,0,1],
        [1,0,0,0,0,0,0,0,0,0,0,1,0,0,1],
        [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
    ];
    pub const SCREEN_WIDTH: u32 = 1920;
    pub const HALF_SCREEN_WIDTH: f32 = SCREEN_WIDTH as f32 / 2.0;
    pub const SCREEN_HEIGHT: u32 = 1080;
    pub const HALF_SCREEN_HEIGHT: f32 = SCREEN_HEIGHT as f32 / 2.0;
    pub const WORLD_WIDTH: u32 = WORLD_LAYOUT[0].len() as u32;
    pub const WORLD_HEIGHT: u32 = WORLD_LAYOUT.len() as u32;
    pub const PHYSICS_FRAME_TIME: f32 = 1.0 / 60.0;
    pub const TILE_SIZE_X_PIXEL: u32 = SCREEN_WIDTH / WORLD_WIDTH;
    pub const TILE_SIZE_Y_PIXEL: u32 = SCREEN_HEIGHT / WORLD_HEIGHT;
    pub const PLAYER_VIEW_ANGLE: f32 = PI / 3.0;
    pub const MAX_VIEW_DISTANCE: u32 = WORLD_WIDTH;
    pub const NUM_RAYS: u32 = SCREEN_WIDTH;
    pub const RAY_PROJECTED_X_SCALE: f32 = SCREEN_WIDTH as f32 / NUM_RAYS as f32;
}
