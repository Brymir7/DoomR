use core::panic;
use std::f32::consts::PI;
use config::config::{ HALF_SCREEN_HEIGHT, HALF_SCREEN_WIDTH, MAP_X_OFFSET, PHYSICS_FRAME_TIME, SCREEN_WIDTH, TILE_SIZE_Y_PIXEL, WORLD_HEIGHT, WORLD_WIDTH };
use macroquad::prelude::*;
pub mod config;

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
        rays: u8,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) -> Vec<RaycastResult> {
        let mut results = Vec::new();
        for i in 0..rays {
            let angle =
                player_angle -
                config::config::PLAYER_VIEW_ANGLE / 2.0 +
                ((i as f32) * config::config::PLAYER_VIEW_ANGLE) / (rays as f32);
            if let Some(result) = RaycastSystem::daa_raycast(origin, angle, tile_map) {
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
        let mut dist_side_x =
            (if direction.x < 0.0 {
                origin.x.trunc() - origin.x
            } else {
                1.0 + origin.x.trunc() - origin.x
            }) * relative_tile_dist_x;
        let mut dist_side_y =
            (if direction.y < 0.0 {
                origin.y - origin.y.trunc()
            } else {
                1.0 + origin.y - origin.y.trunc()
            }) * relative_tile_dist_y;
        let mut curr_map_tile_x = origin.x.trunc() as usize;
        let mut curr_map_tile_y = origin.y.trunc() as usize;
        while
            curr_map_tile_x >= 0 && 
            curr_map_tile_x < WORLD_WIDTH &&
            curr_map_tile_y <= WORLD_HEIGHT &&
            curr_map_tile_y >= 0 
        {
            let is_x_side = dist_side_x < dist_side_y;
            if is_x_side {
                dist_side_x += relative_tile_dist_x;
                curr_map_tile_x = ((curr_map_tile_x as isize) + step_x) as usize;
            } else {
                dist_side_y += relative_tile_dist_y;
                curr_map_tile_y = ((curr_map_tile_y as isize) + step_y) as usize;
            }

            if tile_map[curr_map_tile_y][curr_map_tile_x] == EntityType::Wall {
                let distance = if is_x_side {
                    dist_side_x - relative_tile_dist_x*2.0
                } else {
                    dist_side_y - relative_tile_dist_y  * 2.0
                };
                return Some(RaycastResult {
                    distance,
                    entity_pos: Vec2::new(curr_map_tile_x as f32, curr_map_tile_y as f32),
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
    fn render_rays(player_origin: Vec2, raycast_result: &Vec<RaycastResult>){
        for result in raycast_result.iter() {
            draw_line(
                player_origin.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 + MAP_X_OFFSET,
                player_origin.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                result.entity_pos.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 + MAP_X_OFFSET,
                result.entity_pos.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                1.0,
                WHITE
            );
        }
    }
}
struct RenderPlayerPOV;
impl RenderPlayerPOV {
    fn render(raycast_result: &Vec<RaycastResult>) {
        for (i, result) in raycast_result.iter().enumerate() {
            let wall_height = (TILE_SIZE_Y_PIXEL as f32 / result.distance).min(HALF_SCREEN_HEIGHT);
            let wall_color = match result.entity {
                EntityType::Wall => GREEN,
                _ => WHITE,
            };
            draw_rectangle(
                (i as f32) * config::config::RAY_PROJECTED_X_SCALE,
                config::config::HALF_SCREEN_HEIGHT - wall_height / 2.0,
                config::config::RAY_PROJECTED_X_SCALE,
                wall_height,
                wall_color
            );
        }
    }
}
struct RaycastResult {
    distance: f32,
    entity_pos: Vec2,
    entity: EntityType,
}
struct World {
    world_layout: [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
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
        Self {
            world_layout,
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
        let player_ray_origin = self.player.pos + Vec2::new(0.5, 0.5);
        let raycast_results = RaycastSystem::raycast(
            player_ray_origin,
            self.player.angle,
            config::config::NUM_RAYS as u8,
            &self.world_layout
        );
        RenderMap::render_world_layout(&self.world_layout);
        RenderMap::render_player_on_map(self.player.pos);
        RenderMap::render_rays(player_ray_origin, &raycast_results);
        RenderPlayerPOV::render(&raycast_results);
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
        draw_text(&format!("FPS: {}", get_fps()), 10.0, 10.0, 20.0, WHITE);
        next_frame().await;
    }
}
