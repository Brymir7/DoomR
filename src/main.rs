use core::panic;
use std::{
    cmp::Ordering,
    collections::HashMap,
    f32::consts::PI,
    ops::Range,
    thread::sleep,
    time::Duration,
};
use config::config::{
    HALF_SCREEN_HEIGHT,
    HALF_SCREEN_WIDTH,
    MAP_X_OFFSET,
    MAP_Y_OFFSET,
    PHYSICS_FRAME_TIME,
    PLAYER_FOV,
    SCREEN_HEIGHT,
    SCREEN_WIDTH,
    TILE_SIZE_X_PIXEL,
    TILE_SIZE_Y_PIXEL,
    WORLD_HEIGHT,
    WORLD_WIDTH,
};
use once_cell::sync::Lazy;
use macroquad::{ color, prelude::*, text };
pub mod config;
#[derive(Hash, Eq, PartialEq, Copy, Clone)]
enum Textures {
    Stone,
}

static TEXTURE_TYPE_TO_TEXTURE2D: Lazy<HashMap<Textures, Texture2D>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        Textures::Stone,
        Texture2D::from_file_with_format(
            include_bytes!("textures/stone.png"),
            Some(ImageFormat::Png)
        )
    );
    map
});
fn window_conf() -> Conf {
    Conf {
        window_title: "DoomR".to_owned(),
        window_width: 1920,
        window_height: 1080,
        window_resizable: false,
        high_dpi: true,
        fullscreen: false,
        sample_count: 1,
        ..Default::default()
    }
}
#[derive(Clone, Copy, PartialEq)]
enum EntityType {
    Player,
    Wall,
    None,
}
struct Player {
    pos: Vec2,
    angle: f32,
    vel: Vec2,
}
struct MovementSystem;
impl MovementSystem {
    fn update(positions: &mut Vec<Vec2>, velocities: &Vec<f32>) {
        for (pos, vel) in positions.iter_mut().zip(velocities.iter()) {
            pos.x += vel * PHYSICS_FRAME_TIME;
        }
    }
    fn update_player(player: &mut Player) {
        player.pos += player.vel * PHYSICS_FRAME_TIME;
    }
}
struct WallCollisionSystem;
impl WallCollisionSystem {
    fn update(positions: &mut Vec<Vec2>, walls: &Vec<Vec2>) {
        for pos in positions.iter_mut() {
            for wall in walls.iter() {
                let point_1 = Vec2::new(wall.x + 0.5, wall.y + 0.5);
                let point_2 = Vec2::new(pos.x + 0.5, pos.y + 0.5);
                let distance = point_1.distance(point_2);
                if distance < 1.0 {
                    let normal = (point_2 - point_1).normalize();
                    *pos += normal * (1.0 - distance);
                }
            }
        }
    }
    fn update_player(position: &mut Vec2, walls: &Vec<Vec2>) {
        for wall in walls.iter() {
            let point_1 = Vec2::new(wall.x + 0.5, wall.y + 0.5);
            let point_2 = Vec2::new(position.x + 0.5, position.y + 0.5);

            let distance_x = (point_2.x - point_1.x).abs();
            let distance_y = (point_2.y - point_1.y).abs();

            if distance_x < 1.0 && distance_y < 1.0 {
                if distance_x > distance_y {
                    let normal = Vec2::new(point_2.x - point_1.x, 0.0).normalize();
                    *position += normal * (1.0 - distance_x);
                } else {
                    let normal = Vec2::new(0.0, point_2.y - point_1.y).normalize();
                    *position += normal * (1.0 - distance_y);
                }
            }
        }
    }
}
struct RaycastSystem;
impl RaycastSystem {
    fn raycast(
        origin: Vec2,
        player_angle: f32,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) -> Vec<RaycastResult> {
        let mut results = Vec::new();
        for i in 0..SCREEN_WIDTH {
            let ray_angle =
                player_angle +
                config::config::PLAYER_FOV / 2.0 -
                ((i as f32) / (SCREEN_WIDTH as f32)) * config::config::PLAYER_FOV;

            if let Some(result) = RaycastSystem::daa_raycast(origin, ray_angle, tile_map) {
                results.push(result);
            }
        }
        results
    }
    fn daa_raycast(
        origin: Vec2,
        specific_angle: f32,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) -> Option<RaycastResult> {
        let direction = Vec2::new(specific_angle.cos(), specific_angle.sin());
        let relative_tile_dist_x = 1.0 / direction.x.abs();
        let relative_tile_dist_y = 1.0 / direction.y.abs();
        let step_x: isize = if direction.x > 0.0 { 1 } else { -1 };
        let step_y: isize = if direction.y > 0.0 { 1 } else { -1 };

        let mut curr_map_tile_x = origin.x.trunc() as usize;
        let mut curr_map_tile_y = origin.y.trunc() as usize;
        let mut dist_side_x = if direction.x < 0.0 {
            (origin.x - (curr_map_tile_x as f32)) * relative_tile_dist_x
        } else {
            ((curr_map_tile_x as f32) + 1.0 - origin.x) * relative_tile_dist_x
        };
        let mut dist_side_y = if direction.y < 0.0 {
            (origin.y - (curr_map_tile_y as f32)) * relative_tile_dist_y
        } else {
            ((curr_map_tile_y as f32) + 1.0 - origin.y) * relative_tile_dist_y
        };
        while
            curr_map_tile_x < WORLD_WIDTH && // assume it hits a wall before reaching the end of the map
            curr_map_tile_y <= WORLD_HEIGHT
        {
            let is_x_side = dist_side_x < dist_side_y;
            if is_x_side {
                assert!(curr_map_tile_x > 0);
                dist_side_x += relative_tile_dist_x;
                curr_map_tile_x = ((curr_map_tile_x as isize) + step_x) as usize;
            } else {
                assert!(curr_map_tile_y > 0);
                dist_side_y += relative_tile_dist_y;
                curr_map_tile_y = ((curr_map_tile_y as isize) + step_y) as usize;
            }

            if tile_map[curr_map_tile_y][curr_map_tile_x] == EntityType::Wall {
                let distance = if is_x_side {
                    dist_side_x - relative_tile_dist_x
                } else {
                    dist_side_y - relative_tile_dist_y
                };
                return Some(RaycastResult {
                    distance,
                    intersection_pos: Vec2::new(
                        origin.x + direction.x * distance,
                        origin.y + direction.y * distance
                    ),
                    hit_from_x_side: is_x_side,
                    entity: EntityType::Wall,
                });
            }
        }
        None
    }
}
struct RenderMap;
impl RenderMap {
    fn render_world_layout(world_layout: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]) {
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                match world_layout[y][x] {
                    EntityType::Wall => {
                        draw_rectangle(
                            (x as f32) * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 +
                                MAP_X_OFFSET,
                            (y as f32) * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                            (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25,
                            (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                            RED
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    fn render_player_on_map(player_pos: Vec2) {
        draw_rectangle(
            player_pos.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 + MAP_X_OFFSET,
            player_pos.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
            (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25,
            (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
            BLUE
        );
    }
    fn render_rays(player_origin: Vec2, raycast_result: &Vec<RaycastResult>) {
        for result in raycast_result.iter() {
            draw_line(
                player_origin.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 + MAP_X_OFFSET,
                player_origin.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                result.intersection_pos.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 +
                    MAP_X_OFFSET,
                result.intersection_pos.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                1.0,
                WHITE
            );
            // draw_circle(
            //     result.intersection_pos.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 +
            //         MAP_X_OFFSET,
            //     result.intersection_pos.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
            //     2.0,
            //     WHITE
            // );
        }
    }
}
struct RenderPlayerPOV;
impl RenderPlayerPOV {
    fn render_floor(
        player_angle: f32,
        player_pos: Vec2,
        floor_layout: &[[Textures; WORLD_WIDTH]; WORLD_HEIGHT],
    ) {
        
        const HALF_PLAYER_FOV: f32 = PLAYER_FOV / 2.0;
        let left_most_ray_dir = Vec2::new(
            (player_angle - HALF_PLAYER_FOV).cos(),
            (player_angle - HALF_PLAYER_FOV).sin(),
        );
        let right_most_ray_dir = Vec2::new(
            (player_angle + HALF_PLAYER_FOV).cos(),
            (player_angle + HALF_PLAYER_FOV).sin(),
        );
    
        let floor_step = (right_most_ray_dir - left_most_ray_dir) / SCREEN_WIDTH as f32;
        for row in HALF_SCREEN_HEIGHT as usize..SCREEN_HEIGHT {
            let row_distance = HALF_SCREEN_HEIGHT / (row as f32 - HALF_SCREEN_HEIGHT).max(0.01); // near  screen middle the distance approaches infinity 
            let floor_ray = left_most_ray_dir * row_distance;
            let mut floor_pos = player_pos + floor_ray;
            for col in 0..SCREEN_WIDTH {
                let floor_tile_pos = floor_pos.as_uvec2();
    
                if let Some(&texture_type) = floor_layout
                    .get(floor_tile_pos.y as usize)
                    .and_then(|row| row.get(floor_tile_pos.x as usize))
                {
                    if let Some(texture) = TEXTURE_TYPE_TO_TEXTURE2D.get(&texture_type) {
                        let texture_coords = Vec2::new(
                            floor_pos.x.fract() * texture.width() as f32,
                            floor_pos.y.fract() * texture.height() as f32,
                        );
    
                        // Apply distance-based shading
                        let shade = (1.0 - (row_distance / 100.0)).clamp(0.0, 1.0);
                        let color = Color::new(shade, shade, shade, 1.0);
    
                        draw_texture_ex(
                            texture,
                            col as f32,
                            row as f32,
                            color,
                            DrawTextureParams {
                                source: Some(Rect::new(
                                    texture_coords.x,
                                    texture_coords.y,
                                    1.0,
                                    1.0,
                                )),
                                ..Default::default()
                            },
                        );
                    }
                }
                floor_pos += floor_step * row_distance;
            }
        }
    }
    
    fn render_world(raycast_result: &Vec<RaycastResult>) {
        for (i, result) in raycast_result.iter().enumerate() {
            let wall_height = ((SCREEN_HEIGHT as f32) / (result.distance - 0.5 + 0.000001)).min(
                SCREEN_HEIGHT as f32
            );
            let wall_color = match result.entity {
                EntityType::Wall => GREEN,
                _ => WHITE,
            };
            let shade =
                1.0 - (result.distance / (WORLD_WIDTH.max(WORLD_HEIGHT) as f32)).clamp(0.0, 1.0);
            let wall_color = Color::new(
                wall_color.r * shade,
                wall_color.g * shade,
                wall_color.b * shade,
                1.0
            );
            let wall_color = if result.hit_from_x_side {
                wall_color
            } else {
                Color::new(wall_color.r * 0.8, wall_color.g * 0.8, wall_color.b * 0.8, 1.0)
            };
            draw_rectangle(
                (i as f32) * 1.0,
                config::config::HALF_SCREEN_HEIGHT - wall_height / 2.0,
                1.0,
                wall_height,
                wall_color
            );
        }
    }
}
struct RaycastResult {
    distance: f32,
    hit_from_x_side: bool,
    intersection_pos: Vec2,
    entity: EntityType,
}
struct World {
    world_layout: [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
    floor_layout: [[Textures; WORLD_WIDTH]; WORLD_HEIGHT],
    roof_layout: [[Textures; WORLD_WIDTH]; WORLD_HEIGHT],
    walls: Vec<Vec2>,
    player: Player,
}
impl World {
    fn default() -> Self {
        let mut walls = Vec::new();
        let mut player = Player {
            pos: Vec2::new(0.0, 0.0),
            angle: 0.0,
            vel: Vec2::new(0.0, 0.0),
        };
        let layout = config::config::WORLD_LAYOUT;
        let mut world_layout = [[EntityType::None; WORLD_WIDTH]; WORLD_HEIGHT];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                world_layout[y][x] = match layout[y][x] {
                    0 => EntityType::None,
                    1 => EntityType::Wall,
                    2 => EntityType::Player,
                    _ => panic!("Invalid entity type in world layout"),
                };
                if layout[y][x] == 1 {
                    walls.push(Vec2::new(x as f32, y as f32));
                }
                if layout[y][x] == 2 {
                    if player.pos != Vec2::ZERO {
                        panic!("Multiple player entities in world layout");
                    }
                    player.pos = Vec2::new(x as f32, y as f32);
                }
            }
        }
        let floor_layout = [[Textures::Stone; WORLD_WIDTH]; WORLD_HEIGHT];
        let roof_layout = [[Textures::Stone; WORLD_WIDTH]; WORLD_HEIGHT];
        Self {
            world_layout,
            floor_layout,
            roof_layout,
            walls,
            player,
        }
    }
    fn handle_input(&mut self) {
        if is_key_down(KeyCode::W) {
            self.player.vel = Vec2::new(self.player.angle.cos(), self.player.angle.sin());
        } else if is_key_down(KeyCode::S) {
            self.player.vel = Vec2::new(-self.player.angle.cos(), -self.player.angle.sin());
        } else {
            self.player.vel = Vec2::new(0.0, 0.0);
        }
        if is_key_down(KeyCode::A) {
            self.player.angle -= 0.1;
        }
        if is_key_down(KeyCode::D) {
            self.player.angle += 0.1;
        }
    }

    fn update(&mut self) {
        MovementSystem::update_player(&mut self.player);
        WallCollisionSystem::update_player(&mut self.player.pos, &self.walls);
    }
    fn draw(&self) {
        clear_background(LIGHTGRAY);
        let player_ray_origin = self.player.pos + Vec2::new(0.5, 0.5);
        let start_time = get_time();
        let raycast_results = RaycastSystem::raycast(
            player_ray_origin,
            self.player.angle,
            &self.world_layout
        );
        let end_time = get_time();
        let elapsed_time = end_time - start_time;
        RenderPlayerPOV::render_floor(self.player.angle, player_ray_origin, &self.floor_layout);
        RenderPlayerPOV::render_world(&raycast_results);

        RenderMap::render_world_layout(&self.world_layout);
        RenderMap::render_player_on_map(self.player.pos);
        RenderMap::render_rays(player_ray_origin, &raycast_results);

        draw_text(&format!("Raycasting FPS: {}", 1.0 / elapsed_time), 10.0, 30.0, 20.0, RED);
    }
}
#[macroquad::main(window_conf)]
async fn main() {
    let mut elapsed_time = 0.0;
    let mut world = World::default();
    loop {
        clear_background(BLACK);
        elapsed_time += get_frame_time();
        if elapsed_time > PHYSICS_FRAME_TIME {
            world.handle_input();
            world.update();
            elapsed_time = 0.0;
        }
        world.draw();
        draw_text(&format!("FPS: {}", 1.0 / get_frame_time()), 10.0, 10.0, 20.0, WHITE);
        next_frame().await;
    }
}
