use core::panic;
use std::{collections::HashMap, u8};
use config::config::{
    HALF_SCREEN_HEIGHT,
    MAP_X_OFFSET,
    PHYSICS_FRAME_TIME,
    PLAYER_FOV,
    SCREEN_HEIGHT,
    SCREEN_WIDTH,
    WORLD_HEIGHT,
    WORLD_WIDTH,
};
use image_utils::load_and_convert_texture;
use once_cell::sync::Lazy;
use macroquad::prelude::*;
use shaders::shaders::{ DEFAULT_VERTEX_SHADER, FLOOR_FRAGMENT_SHADER };
pub mod config;
pub mod shaders;
pub mod image_utils;
#[derive(Hash, Eq, PartialEq, Copy, Clone)]
enum Textures {
    Stone,
    Weapon,
}

static TEXTURE_TYPE_TO_TEXTURE2D: Lazy<HashMap<Textures, Texture2D>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        Textures::Stone,
        Texture2D::from_file_with_format(
            include_bytes!("../textures/stone.png"),
            Some(ImageFormat::Png)
        )
    );
    map.insert(
        Textures::Weapon,
        load_and_convert_texture(include_bytes!("../textures/weapon.png"), ImageFormat::Png)
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
#[derive(Clone, Copy, PartialEq, Debug)]
enum EntityType {
    Player,
    Wall(u8),
    None,
    Enemy(u8),
}
enum WorldEventType {
    PlayerHitEnemy,
    EnemyHitPlayer,
}
#[derive(PartialEq, Clone, Copy)]
struct Tile {
    x: u8,
    y: u8,
}
impl Tile {
    fn from_vec2(pos: Vec2) -> Self {
        return Tile {
            x: pos.x.round() as u8,
            y: pos.y.round() as u8,
        };
    }
}
struct WorldEvent {
    event_type: WorldEventType,
    triggered_by_tile_handle: Tile,
    target_tile_handle: Tile,
}
impl WorldEvent {
    fn player_hit_enemy(player_handle: Tile, enemy_handle: Tile) -> Self {
        return WorldEvent {
            event_type: WorldEventType::PlayerHitEnemy,
            triggered_by_tile_handle: player_handle,
            target_tile_handle: enemy_handle,
        };
    }
}
// struct AnimationSystem;

// impl AnimationSystem {
//     fn play_explosion(x: f32, y: f32, radius: f32) {
//         for i in 0..16 {
//             let angle = ((i as f32) * (std::f32::consts::PI * 2.0)) / 16.0;
//             let dx = radius * angle.cos();
//             let dy = radius * angle.sin();
//             let w = radius / 8.0;
//             let h = radius / 8.0;
//             draw_rectangle(x + dx, y + dy, w, h, Color::from_rgba(255, 128, 0, 255));
//         }
//     }
// }
struct Enemies {
    positions: Vec<Vec2>,
    angles: Vec<f32>,
    velocities: Vec<Vec2>,
    healths: Vec<u8>,
    sizes: Vec<Vec2>,
}
struct EnemyInformation {
    pos: Vec2,
    angle: f32,
    vel: Vec2,
    health: u8,
    size: Vec2,
}
impl Enemies {
    fn new_enemy(&mut self, pos: Vec2, angle: f32, velocity: Vec2, health: u8, size: Vec2) -> usize {
        self.positions.push(pos);
        self.angles.push(angle);
        self.velocities.push(velocity);
        self.healths.push(health);
        self.sizes.push(size);
        return self.positions.len();
    }
    fn destroy_enemy(&mut self, idx: u8) {
        self.positions.swap_remove(idx as usize);
        self.angles.swap_remove(idx as usize);
        self.velocities.swap_remove(idx as usize);
        self.healths.swap_remove(idx as usize);
    }
    fn get_enemy_information(&self, idx: u8) -> EnemyInformation {
        let idx = idx as usize;
        EnemyInformation {
            pos: *self.positions.get(idx).expect("Tried to acccess invalid enemy idx"),
            angle: *self.angles.get(idx).expect("Tried to acccess invalid enemy idx"),
            vel: *self.velocities.get(idx).expect("Tried to acccess invalid enemy idx"),
            health: *self.healths.get(idx).expect("Tried to acccess invalid enemy idx"),
            size: *self.sizes.get(idx).expect("Tried to acccess invalid enemy idx")
        }
    }
}
struct Player {
    pos: Vec2,
    angle: f32,
    vel: Vec2,
    health: u8,
}
impl Player {
    fn shoot(&self, world_layout: [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT], enemies: &Enemies) -> Option<WorldEvent> {
        let result = RaycastSystem::daa_raycast(self.pos, self.angle, &world_layout, enemies);
        match result.enemy {
            Some(object_hit) => {
                match object_hit.entity {
                    EntityType::Enemy(_) => {
                        if object_hit.distance > 5.0 {
                            return None;
                        }
                        return Some(
                            WorldEvent::player_hit_enemy(
                                Tile::from_vec2(self.pos),
                                Tile::from_vec2(object_hit.intersection_pos)
                            )
                        );
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
struct MovementSystem;

impl MovementSystem {
    fn update(
        positions: &mut Vec<Vec2>,
        velocities: &Vec<Vec2>,
        entity_type: EntityType,
        walls: &Vec<Vec2>,
        world_layout: &mut [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) {
        for ((i, pos), vel) in positions.iter_mut().enumerate().zip(velocities.iter()) {
            let prev_tile = Tile::from_vec2(*pos);
            pos.x += vel.x * PHYSICS_FRAME_TIME;
            pos.y += vel.y * PHYSICS_FRAME_TIME;

            Self::resolve_wall_collisions(pos, walls);

            let new_tile = Tile::from_vec2(*pos);
            match entity_type {
                EntityType::Enemy(_) => {
                    world_layout[new_tile.y as usize][new_tile.x as usize] = EntityType::Enemy(
                        i as u8
                    );
                }
                _ => {
                    panic!("Do not use update function except for enemies atm!");
                }
            }
            if prev_tile != new_tile {
                assert!(
                    matches!(
                        world_layout[prev_tile.y as usize][prev_tile.x as usize],
                        EntityType::Enemy(_)
                    )
                );
                world_layout[prev_tile.y as usize][prev_tile.x as usize] = EntityType::None;
            }
        }
    }
    fn update_enemies(
        enemies: &mut Enemies,
        walls: &Vec<Vec2>,
        world_layout: &mut [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) {
        for (id, (pos_and_vel, size)) in enemies.positions.iter_mut()
            .zip(enemies.velocities.iter())
            .zip(enemies.sizes.iter())
            .enumerate()
        {
            let pos = pos_and_vel.0;
            let vel = pos_and_vel.1;
            let prev_tiles = Self::get_occupied_tiles(*pos, *size);
            pos.x += vel.x * PHYSICS_FRAME_TIME;
            pos.y += vel.y * PHYSICS_FRAME_TIME;

            Self::resolve_wall_collisions(pos, walls);

            let new_tiles = Self::get_occupied_tiles(*pos, *size);

            // Remove enemy from previous tiles
            for tile in prev_tiles {
                world_layout[tile.y as usize][tile.x as usize] = EntityType::None;
            }

            // Add enemy to new tiles
            for tile in new_tiles {
                world_layout[tile.y as usize][tile.x as usize] = EntityType::Enemy(id as u8);
            }
        }
    }

    fn get_occupied_tiles(pos: Vec2, size: Vec2) -> Vec<Tile> {
        let mut tiles = Vec::new();
        let start_x = pos.x.floor() as u8;
        let start_y = pos.y.floor() as u8;
        let end_x = (pos.x + size.x - 0.01).floor() as u8;
        let end_y = (pos.y + size.y - 0.01).floor() as u8;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                tiles.push(Tile { x, y });
            }
        }
        tiles
    }

    fn update_player(
        player: &mut Player,
        walls: &Vec<Vec2>,
        world_layout: &mut [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) {
        let prev_tile = Tile::from_vec2(player.pos);
        player.pos += player.vel * PHYSICS_FRAME_TIME;
        Self::resolve_wall_collisions(&mut player.pos, walls);
        let new_tile = Tile::from_vec2(player.pos);
        world_layout[new_tile.y as usize][new_tile.x as usize] = EntityType::Player;
        if prev_tile != new_tile {
            assert!(world_layout[prev_tile.y as usize][prev_tile.x as usize] == EntityType::Player);
            world_layout[prev_tile.y as usize][prev_tile.x as usize] = EntityType::None;
        }
    }

    fn resolve_wall_collisions(position: &mut Vec2, walls: &Vec<Vec2>) {
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
struct RaycastStepResult { // to avoid raytracing twice, we raytrace -  add any enemy we find, and break only at a wall (so that we can render enemy in front of wall)
    block: Option<RaycastResult>,
    enemy: Option<RaycastResult>,
}
struct RaycastSystem;
impl RaycastSystem {
    fn raycast(
        origin: Vec2,
        player_angle: f32,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemies: &Enemies,
    ) -> Vec<RaycastStepResult> {
        let mut res = Vec::new();
        for i in 0..SCREEN_WIDTH {
            let ray_angle =
                player_angle +
                config::config::PLAYER_FOV / 2.0 -
                ((i as f32) / (SCREEN_WIDTH as f32)) * config::config::PLAYER_FOV;

            let step_result = RaycastSystem::daa_raycast(origin, ray_angle, tile_map, enemies);
            res.push(step_result);
        }
        res
    }
    fn daa_raycast(
        origin: Vec2,
        specific_angle: f32,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemies: &Enemies,
    ) -> RaycastStepResult {
        let mut raycast_step_res = RaycastStepResult {
            block: None,
            enemy: None,
        };
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

            match tile_map[curr_map_tile_y][curr_map_tile_x] {
                EntityType::Wall(handle) => {
                    let distance = if is_x_side {
                        dist_side_x - relative_tile_dist_x
                    } else {
                        dist_side_y - relative_tile_dist_y
                    };
                    raycast_step_res.block = Some(RaycastResult {
                        distance,
                        intersection_pos: Vec2::new(
                            origin.x + direction.x * distance,
                            origin.y + direction.y * distance
                        ),
                        hit_from_x_side: is_x_side,
                        entity: EntityType::Wall(handle),
                        map_idx: Tile {
                            x: curr_map_tile_x as u8,
                            y: curr_map_tile_y as u8,
                        }
                    });
                    break;
                }
                EntityType::Enemy(id) => {
                    let enemy_pos = enemies.positions[id as usize];
                    let enemy_size = enemies.sizes[id as usize];
                    let distance = Self::calculate_enemy_distance(origin, direction, enemy_pos, enemy_size);
                    if distance < raycast_step_res.block.map_or(f32::INFINITY, |e| e.distance) {
                        raycast_step_res.enemy = Some(RaycastResult {
                            distance,
                            intersection_pos: origin + direction * distance,
                            hit_from_x_side: is_x_side,
                            entity: EntityType::Enemy(id),
                            map_idx: Tile {
                                x: curr_map_tile_x as u8,
                                y: curr_map_tile_y as u8,
                            },
                        });
                    }
                    // don't break here because we want to still see the background behind the enemy
                }
                _ => {}
            }
        }
        raycast_step_res
    }
    fn calculate_enemy_distance(origin: Vec2, direction: Vec2, enemy_pos: Vec2, enemy_size: Vec2) -> f32 {
        let t_near_x = (enemy_pos.x - origin.x) / direction.x;
        let t_near_y = (enemy_pos.y - origin.y) / direction.y;
        let t_far_x = (enemy_pos.x + enemy_size.x - origin.x) / direction.x;
        let t_far_y = (enemy_pos.y + enemy_size.y - origin.y) / direction.y;

        let t_near = t_near_x.max(t_near_y);
        let t_far = t_far_x.min(t_far_y);

        if t_near < t_far && t_far > 0.0 {
            t_near
        } else {
            f32::INFINITY
        }
    }
}
struct RenderMap;
impl RenderMap {
    fn render_world_layout(world_layout: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]) {
        draw_rectangle(MAP_X_OFFSET, 0.0, (SCREEN_WIDTH as f32) - MAP_X_OFFSET, 270.0, GRAY);
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                match world_layout[y][x] {
                    EntityType::Wall(_) => {
                        draw_rectangle(
                            (x as f32) * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 +
                                MAP_X_OFFSET,
                            (y as f32) * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                            (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25,
                            (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                            BROWN
                        );
                    }
                    EntityType::Enemy(_) => {
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
    fn render_rays(player_origin: Vec2, raycast_result: &Vec<RaycastStepResult>) {
        for result in raycast_result.iter() {
            if let Some(block) = &result.block {
                draw_line(
                    player_origin.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 +
                        MAP_X_OFFSET,
                    player_origin.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                    block.intersection_pos.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 +
                        MAP_X_OFFSET,
                    block.intersection_pos.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                    1.0,
                    WHITE
                );
            }
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
    fn render_floor(material: &Material, player_angle: f32, player_pos: Vec2) {
        const HALF_PLAYER_FOV: f32 = PLAYER_FOV / 2.0;
        let left_most_ray_dir = Vec2::new(
            (player_angle - HALF_PLAYER_FOV).cos(),
            (player_angle - HALF_PLAYER_FOV).sin()
        );
        let right_most_ray_dir = Vec2::new(
            (player_angle + HALF_PLAYER_FOV).cos(),
            (player_angle + HALF_PLAYER_FOV).sin()
        );
        material.set_uniform("u_player_pos", player_pos);
        material.set_uniform("u_left_ray_dir", left_most_ray_dir);
        material.set_uniform("u_right_ray_dir", right_most_ray_dir);
        material.set_uniform("u_half_screen_height", HALF_SCREEN_HEIGHT as f32);
        material.set_uniform("u_screen_width", SCREEN_WIDTH as f32);
        material.set_uniform("u_screen_height", SCREEN_HEIGHT as f32);
        material.set_texture(
            "u_floor_texture",
            TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::Stone)
                .expect("Couldnt load stone texture")
                .clone()
        );
        gl_use_material(&material);
        material.set_uniform("is_ceiling", 1.0 as f32);
        draw_rectangle(
            0.0,
            0.0,
            SCREEN_WIDTH as f32,
            HALF_SCREEN_HEIGHT as f32,
            Color::from_rgba(255, 255, 255, 255)
        );
        material.set_uniform("is_ceiling", -1.0 as f32);
        draw_rectangle(
            0.0,
            HALF_SCREEN_HEIGHT,
            SCREEN_WIDTH as f32,
            HALF_SCREEN_HEIGHT as f32,
            Color::from_rgba(255, 255, 255, 255)
        );
        gl_use_default_material();
    }

    fn render_world(
        player_origin: Vec2,
        raycast_step_res: &Vec<RaycastStepResult>,
        enemies_positions: &Vec<Vec2>,
        world_layout: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
    ) {
        for (i, result) in raycast_step_res.iter().enumerate() {
            if let Some(block) = &result.block {
                let wall_color = match block.entity {
                    EntityType::Wall(_) => GREEN,
                    _ => panic!("Non wall block"),
                };
                let wall_height = ((SCREEN_HEIGHT as f32) / (block.distance - 0.5 + 0.000001)).min(
                    SCREEN_HEIGHT as f32
                );
                let shade =
                    1.0 - (block.distance / (WORLD_WIDTH.max(WORLD_HEIGHT) as f32)).clamp(0.0, 1.0);
                let wall_color = Color::new(
                    wall_color.r * shade,
                    wall_color.g * shade,
                    wall_color.b * shade,
                    1.0
                );
                let wall_color = if block.hit_from_x_side {
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
            if let Some(enemy) = &result.enemy {
                let enemy_handle = world_layout[enemy.map_idx.y as usize][enemy.map_idx.x as usize];
                let enemy_pos = match enemy_handle {
                    EntityType::Enemy(idx) => {
                        enemies_positions[idx as usize]
                    }
                    _ => panic!("Invalid enemy handle")
                };
                let distance = player_origin.distance(enemy_pos);
                let wall_color = match enemy.entity {
                    EntityType::Enemy(_) => RED,
                    _ => panic!("Non enemy block"),
                };
                let wall_height = (
                    (SCREEN_HEIGHT as f32) /
                    (distance * 1.5 - 0.5 + 0.000001)
                ).min(SCREEN_HEIGHT as f32);
                let shade =
                    1.0 - (distance / (WORLD_WIDTH.max(WORLD_HEIGHT) as f32)).clamp(0.0, 1.0);
                let wall_color = Color::new(
                    wall_color.r * shade,
                    wall_color.g * shade,
                    wall_color.b * shade,
                    1.0
                );
                let wall_color = if enemy.hit_from_x_side {
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
    fn render_weapon() {
        let weapon_texture = TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::Weapon).expect(
            "Failed to load weapon sprite"
        );
        draw_texture_ex(
            weapon_texture,
            (SCREEN_WIDTH as f32) * 0.5 - weapon_texture.width() * 0.5,
            (SCREEN_HEIGHT as f32) * 0.85 - weapon_texture.height(),
            Color::from_rgba(255, 255, 255, 255),
            DrawTextureParams {
                dest_size: Some(
                    Vec2::new(weapon_texture.width() * 2.0, weapon_texture.height() * 2.0)
                ),
                ..Default::default()
            }
        )
    }
}
#[derive(Clone, Copy)]
struct RaycastResult {
    distance: f32,
    hit_from_x_side: bool,
    intersection_pos: Vec2,
    entity: EntityType,
    map_idx: Tile,
}
struct World {
    world_layout: [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
    background_material: Material,
    walls: Vec<Vec2>,
    enemies: Enemies,
    player: Player,
}
impl World {
    fn default() -> Self {
        let mut walls = Vec::new();
        let mut enemies = Enemies {
            angles: Vec::new(),
            positions: Vec::new(),
            velocities: Vec::new(),
            healths: Vec::new(),
            sizes: Vec::new(),
        };
        let mut player = Player {
            pos: Vec2::new(0.0, 0.0),
            angle: 0.0,
            vel: Vec2::new(0.0, 0.0),
            health: 3,
        };
        let layout = config::config::WORLD_LAYOUT;
        let mut world_layout = [[EntityType::None; WORLD_WIDTH]; WORLD_HEIGHT];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                match layout[y][x] {
                    0 => {
                        world_layout[y][x] = EntityType::None;
                    }
                    1 => {
                        world_layout[y][x] = EntityType::Wall(walls.len() as u8);
                        walls.push(Vec2::new(x as f32, y as f32));
                    }
                    2 => {
                        world_layout[y][x] = EntityType::Player;
                        if player.pos != Vec2::ZERO {
                            panic!("Multiple player entities in world layout");
                        }
                        player.pos = Vec2::new(x as f32, y as f32);
                    }
                    3 => {
                        let handle = enemies.new_enemy(
                            Vec2::new(x as f32, y as f32),
                            0.0,
                            Vec2::new(1.0, -1.0),
                            1,
                            Vec2::new(1.0, 1.0)
                        );
                        world_layout[y][x] = EntityType::Enemy(handle as u8);
                    }
                    _ => panic!("Invalid entity type in world layout"),
                };
            }
        }

        let material = load_material(
            ShaderSource::Glsl {
                vertex: &DEFAULT_VERTEX_SHADER,
                fragment: &FLOOR_FRAGMENT_SHADER,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc {
                        name: "u_player_pos".to_string(),
                        uniform_type: UniformType::Float2,
                        array_count: 1,
                    },
                    UniformDesc {
                        name: "u_left_ray_dir".to_string(),
                        uniform_type: UniformType::Float2,
                        array_count: 1,
                    },
                    UniformDesc {
                        name: "u_right_ray_dir".to_string(),
                        uniform_type: UniformType::Float2,
                        array_count: 1,
                    },
                    UniformDesc {
                        name: "u_half_screen_height".to_string(),
                        uniform_type: UniformType::Float1,
                        array_count: 1,
                    },
                    UniformDesc {
                        name: "u_screen_width".to_string(),
                        uniform_type: UniformType::Float1,
                        array_count: 1,
                    },
                    UniformDesc {
                        name: "u_screen_height".to_string(),
                        uniform_type: UniformType::Float1,
                        array_count: 1,
                    },
                    UniformDesc {
                        name: "is_ceiling".to_string(),
                        uniform_type: UniformType::Float1,
                        array_count: 1,
                    }
                ],
                textures: vec!["u_floor_texture".to_string()],
                ..Default::default()
            }
        ).unwrap();
        Self {
            world_layout,
            background_material: material,
            walls,
            enemies,
            player,
        }
    }
    fn kill_enemy(&mut self, enemy_tile: Tile, enemy_idx: u8) {
        self.enemies.destroy_enemy(enemy_idx);
        self.world_layout[enemy_tile.y as usize][enemy_tile.x as usize] = EntityType::None;

    }
    fn handle_game_event(&mut self, event: WorldEvent) {
        match event.event_type {
            WorldEventType::PlayerHitEnemy => {
                let enemy_handle: EntityType = self.world_layout[event.target_tile_handle.y as usize][event.target_tile_handle.x as usize];
                match enemy_handle {
                    EntityType::Enemy(idx) => {
                        let health = self.enemies.healths.get_mut(idx as usize).expect("Invalid handle in world layout");
                        *health -= 1;
                        if *health == 0 {
                            self.kill_enemy(event.target_tile_handle, idx);
                        }
                    }
                    _ => panic!("Hit invalid enemy")
                }
            }
            _ => panic!("Unahndled game event")
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
            self.player.angle -= 0.75 * get_frame_time();
        }
        if is_key_down(KeyCode::D) {
            self.player.angle += 0.75 * get_frame_time();
        }
        if is_key_pressed(KeyCode::Space) {
            let game_event = self.player.shoot(self.world_layout, &self.enemies);
            if let Some(event) = game_event {
                self.handle_game_event(event);
            }
        }   
    }

    fn update(&mut self) {
        assert!(self.enemies.positions.len() < 255);
        assert!(self.world_layout.len() < 255 && self.world_layout[0].len() < 255);
        assert!(self.walls.len() < 255);
        MovementSystem::update_player(&mut self.player, &self.walls, &mut self.world_layout);
        MovementSystem::update_enemies(
            &mut self.enemies,
            &self.walls,
            &mut self.world_layout
        );
    }
    fn draw(&self) {
        clear_background(LIGHTGRAY);
        let player_ray_origin = self.player.pos + Vec2::new(0.5, 0.5);
        let start_time = get_time();
        let raycast_result: Vec<RaycastStepResult> = RaycastSystem::raycast(
            player_ray_origin,
            self.player.angle,
            &self.world_layout,
            &self.enemies,
        );
        let end_time = get_time();
        let elapsed_time = end_time - start_time;
        RenderPlayerPOV::render_floor(
            &self.background_material,
            self.player.angle,
            player_ray_origin
        );
        RenderPlayerPOV::render_world(self.player.pos, &raycast_result, &self.enemies.positions, &self.world_layout);
        RenderPlayerPOV::render_weapon();
        RenderMap::render_world_layout(&self.world_layout);
        RenderMap::render_player_on_map(self.player.pos);
        RenderMap::render_rays(player_ray_origin, &raycast_result);
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
        world.handle_input();
        if elapsed_time > PHYSICS_FRAME_TIME {
            world.update();
            elapsed_time = 0.0;
        }
        world.draw();
        draw_text(&format!("FPS: {}", 1.0 / get_frame_time()), 10.0, 10.0, 20.0, WHITE);
        next_frame().await;
    }
}
