use core::panic;
use std::{ collections::HashMap, process::id };
use config::config::{
    AMOUNT_OF_RAYS,
    HALF_SCREEN_HEIGHT,
    MAP_X_OFFSET,
    PHYSICS_FRAME_TIME,
    PLAYER_FOV,
    RAY_VERTICAL_STRIPE_WIDTH,
    SCREEN_HEIGHT,
    SCREEN_WIDTH,
    TILE_SIZE_X_PIXEL,
    TILE_SIZE_Y_PIXEL,
    WORLD_HEIGHT,
    WORLD_WIDTH,
};
use image_utils::load_and_convert_texture;
use miniquad::date;
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
    SkeletonFrontSpriteSheet,
    SkeletonBackSpriteSheet,
    SkeletonSideSpriteSheet,
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
    map.insert(
        Textures::SkeletonFrontSpriteSheet,
        load_and_convert_texture(
            include_bytes!("../textures/SkeletonFrontSpriteSheet.png"),
            ImageFormat::Png
        )
    );
    map.insert(
        Textures::SkeletonSideSpriteSheet,
        load_and_convert_texture(
            include_bytes!("../textures/SkeletonSideSpriteSheet.png"),
            ImageFormat::Png
        )
    );
    map.insert(
        Textures::SkeletonBackSpriteSheet,
        load_and_convert_texture(
            include_bytes!("../textures/SkeletonBackSpriteSheet.png"),
            ImageFormat::Png
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
#[derive(Clone, Copy)]
enum AnimationCallbackEventType {
    None,
    KillEnemy,
}
#[derive(Clone, Copy)]
struct AnimationCallbackEvent {
    event_type: AnimationCallbackEventType,
    target_handle: u8,
}
impl AnimationCallbackEvent {
    fn none() -> Self {
        AnimationCallbackEvent {
            event_type: AnimationCallbackEventType::None,
            target_handle: 0,
        }
    }
}
struct AnimationSprite {
    source: Rect, // what to sample from spritesheet
    color: Color,
}
#[derive(Clone)]
struct AnimationState {
    frame: u8,
    frames_amount: u8,
    spritesheet_offset_per_frame: Vec2,
    animation_type: EnemyAnimationType,
    sprite_sheet: Texture2D,
    color: Color,
    physics_frames_per_update: f32,
    elapsed_time: f32,
    callback_event: AnimationCallbackEvent,
}
impl AnimationState {
    fn default_skeleton() -> Self {
        let texture = TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::SkeletonFrontSpriteSheet).expect(
            "Failed to load Skeleton Front Spritesheet"
        );
        const FRAMES_AMOUNT: u8 = 3;
        let single_sprite_dimension_x = texture.width() / (FRAMES_AMOUNT as f32);
        AnimationState {
            frame: 0,
            frames_amount: 3,
            spritesheet_offset_per_frame: Vec2::new(single_sprite_dimension_x, 0.0),
            sprite_sheet: texture.clone(),
            color: WHITE,
            animation_type: EnemyAnimationType::SkeletonFront,
            physics_frames_per_update: 20.0 * PHYSICS_FRAME_TIME,
            elapsed_time: 0.0,
            callback_event: AnimationCallbackEvent::none(),
        }
    }
    fn set_physics_frames_per_update(&mut self, frames: f32) {
        self.physics_frames_per_update = frames * PHYSICS_FRAME_TIME;
    }
    fn reset_frames(&mut self) {
        self.frame = 0;
        self.elapsed_time = 0.0;
    }
    fn set_callback(&mut self, callback: AnimationCallbackEvent) {
        self.callback_event = callback;
        self.reset_frames();
    }
    fn next(&mut self, dt: f32) -> AnimationCallbackEvent {
        assert!(self.physics_frames_per_update >= dt);
        self.elapsed_time += dt;
        let mut callback_event = AnimationCallbackEvent::none();
        if self.elapsed_time > self.physics_frames_per_update {
            if self.frame == self.frames_amount - 1 {
                callback_event = self.callback_event;
                println!("Returning callback to kill");
            }
            self.frame = (self.frame + 1) % self.frames_amount;
            self.elapsed_time = 0.0;
        }
        return callback_event;
    }
    fn get_current_sprite_source(&self) -> AnimationSprite {
        let current_frames_offset = (self.frame as f32) * self.spritesheet_offset_per_frame;
        return AnimationSprite {
            source: Rect {
                x: current_frames_offset.x,
                y: current_frames_offset.y,
                w: self.spritesheet_offset_per_frame.x,
                h: self.spritesheet_offset_per_frame.y,
            },
            color: self.color,
        };
    }
    fn change_animation(
        &mut self,
        new_spritesheet: Texture2D,
        new_animation_type: EnemyAnimationType,
        single_sprite_dimensions: Vec2
    ) {
        self.frame = 0;
        self.frames_amount = (new_spritesheet.width() / single_sprite_dimensions.x).trunc() as u8;
        self.spritesheet_offset_per_frame = Vec2::new(
            single_sprite_dimensions.x,
            single_sprite_dimensions.y
        );
        self.sprite_sheet = new_spritesheet;
        self.animation_type = new_animation_type;
    }
}
#[derive(Clone, Copy, PartialEq)]
enum EnemyAnimationType {
    SkeletonFront,
    SkeletonSide,
    SkeletonBack,
}

struct UpdateEnemyAnimation;
impl UpdateEnemyAnimation {
    fn update(
        player_origin: Vec2,
        player_angle: f32,
        enemy_positions: &Vec<Vec2>,
        velocities: &Vec<Vec2>,
        animation_states: &mut Vec<AnimationState>
    ) -> Vec<AnimationCallbackEvent> {
        let mut res: Vec<AnimationCallbackEvent> = Vec::new();
        for ((&enemy_pos, &velocity), animation_state) in enemy_positions
            .iter()
            .zip(velocities.iter())
            .zip(animation_states.iter_mut()) {
            let to_player = player_origin - enemy_pos;
            let enemy_angle = velocity.angle_between(to_player);
            let callback_event = animation_state.next(PHYSICS_FRAME_TIME);
            res.push(callback_event);
            match enemy_angle.abs() {
                angle if angle < std::f32::consts::FRAC_PI_4 => {
                    if animation_state.animation_type != EnemyAnimationType::SkeletonFront {
                        animation_state.change_animation(
                            TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::SkeletonFrontSpriteSheet)
                                .expect("Failed to load spritesheet skeleton")
                                .clone(),
                            EnemyAnimationType::SkeletonFront,
                            Vec2::new(31.0, 48.0)
                        );
                    }
                }
                angle if angle > 3.0 * std::f32::consts::FRAC_PI_4 => {
                    if animation_state.animation_type != EnemyAnimationType::SkeletonBack {
                        animation_state.change_animation(
                            TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::SkeletonBackSpriteSheet)
                                .expect("Failed to load spritesheet skeleton")
                                .clone(),
                            EnemyAnimationType::SkeletonBack,
                            Vec2::new(31.0, 48.0)
                        );
                    }
                }
                _ => {
                    if animation_state.animation_type != EnemyAnimationType::SkeletonSide {
                        animation_state.change_animation(
                            TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::SkeletonSideSpriteSheet)
                                .expect("Failed to load spritesheet skeleton")
                                .clone(),
                            EnemyAnimationType::SkeletonSide,
                            Vec2::new(31.0, 48.0)
                        );
                    }
                }
            };
        }
        res
    }
}
struct CallbackHandler;
impl CallbackHandler {
    fn handle_animation_callbacks(
        callbacks: Vec<AnimationCallbackEvent>,
        world_layout: &mut [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemies: &mut Enemies
    ) {
        for callback in callbacks {
            match callback.event_type {
                AnimationCallbackEventType::KillEnemy => {
                    let enemy_idx = callback.target_handle;
                    let mut enemy_information = enemies.get_enemy_information(enemy_idx);
                    enemy_information.animation_state.animation_type =
                        EnemyAnimationType::SkeletonBack;
                    let enemy_pos = enemy_information.pos;
                    let enemy_size = enemy_information.size;
                    let start_tile_x = enemy_pos.x.floor() as usize;
                    let start_tile_y = enemy_pos.y.floor() as usize;
                    let end_tile_x = (enemy_pos.x + enemy_size.x).ceil() as usize;
                    let end_tile_y = (enemy_pos.y + enemy_size.y).ceil() as usize;

                    for y in start_tile_y..end_tile_y {
                        for x in start_tile_x..end_tile_x {
                            if y < world_layout.len() && x < world_layout[y].len() {
                                if let EntityType::Enemy(id) = world_layout[y][x] {
                                    if id == enemy_idx {
                                        world_layout[y][x] = EntityType::None;
                                    }
                                }
                            }
                        }
                    }
                    enemies.destroy_enemy(enemy_idx);
                }
                AnimationCallbackEventType::None => {}
            }
        }
    }
}
struct Enemies {
    positions: Vec<Vec2>,
    velocities: Vec<Vec2>,
    healths: Vec<u8>,
    sizes: Vec<Vec2>,
    animation_states: Vec<AnimationState>,
}
struct EnemyInformation {
    idx: u8,
    pos: Vec2,
    vel: Vec2,
    health: u8,
    size: Vec2,
    animation_state: AnimationState,
}
impl Enemies {
    fn new_enemy(
        &mut self,
        pos: Vec2,
        velocity: Vec2,
        health: u8,
        size: Vec2,
        animation: AnimationState
    ) -> usize {
        self.positions.push(pos);
        self.velocities.push(velocity);
        self.healths.push(health);
        self.sizes.push(size);
        self.animation_states.push(animation);
        return self.positions.len() - 1;
    }
    fn destroy_enemy(&mut self, idx: u8) {
        self.positions.swap_remove(idx as usize);
        self.velocities.swap_remove(idx as usize);
        self.healths.swap_remove(idx as usize);
    }
    fn get_enemy_information(&self, idx: u8) -> EnemyInformation {
        let idx = idx as usize;
        println!("{}, len enemies {}", idx, self.positions.len());
        EnemyInformation {
            idx: idx as u8,
            pos: *self.positions.get(idx).expect("Tried to acccess invalid enemy idx"),
            vel: *self.velocities.get(idx).expect("Tried to acccess invalid enemy idx"),
            health: *self.healths.get(idx).expect("Tried to acccess invalid enemy idx"),
            size: *self.sizes.get(idx).expect("Tried to acccess invalid enemy idx"),
            animation_state: self.animation_states
                .get(idx)
                .expect("Tried to acccess invalid enemy idx")
                .clone(),
        }
    }
    fn update_based_on_enemy_information(&mut self, enemy_information: EnemyInformation) {
        let idx = enemy_information.idx as usize;
        *self.positions.get_mut(idx).expect("Invalid enemy information update") =
            enemy_information.pos;
        *self.velocities.get_mut(idx).expect("Invalid enemy information update") =
            enemy_information.vel;
        *self.healths.get_mut(idx).expect("Invalid enemy information update") =
            enemy_information.health;
        *self.sizes.get_mut(idx).expect("Invalid enemy information update") =
            enemy_information.size;
        *self.animation_states.get_mut(idx).expect("Invalid enemy information update") =
            enemy_information.animation_state;
    }
}
struct Player {
    pos: Vec2,
    angle: f32,
    vel: Vec2,
    health: u8,
}
impl Player {
    fn shoot(
        &self,
        world_layout: [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemies: &Enemies
    ) -> Option<WorldEvent> {
        const RAY_SPREAD: f32 = PLAYER_FOV / 2.0 / 10.0; // basically defines the hitbox of the player shooting
        let angles = [self.angle - RAY_SPREAD, self.angle, self.angle + RAY_SPREAD];

        for &angle in &angles {
            let result = RaycastSystem::daa_raycast(self.pos, angle, &world_layout, enemies);

            match result.enemy {
                Some(object_hit) => {
                    match object_hit.entity {
                        EntityType::Enemy(_) => {
                            if object_hit.distance <= 5.0 {
                                // defines max distance to be able to shoot
                                return Some(
                                    WorldEvent::player_hit_enemy(
                                        Tile::from_vec2(self.pos),
                                        Tile::from_vec2(object_hit.intersection_pos)
                                    )
                                );
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        None
    }
}

struct MovementSystem;
impl MovementSystem {
    fn update_enemies(
        enemies: &mut Enemies,
        walls: &Vec<Vec2>,
        world_layout: &mut [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) {
        for (id, (pos_and_vel, size)) in enemies.positions
            .iter_mut()
            .zip(enemies.velocities.iter())
            .zip(enemies.sizes.iter())
            .enumerate() {
            let pos = pos_and_vel.0;
            let vel = pos_and_vel.1;
            let prev_tiles = Self::get_occupied_tiles(*pos, *size);
            pos.x += vel.x * PHYSICS_FRAME_TIME;
            pos.y += vel.y * PHYSICS_FRAME_TIME;

            Self::resolve_wall_collisions(pos, walls);
            let new_tiles = Self::get_occupied_tiles(*pos, *size);
            for tile in prev_tiles {
                world_layout[tile.y as usize][tile.x as usize] = EntityType::None;
            }
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
struct RaycastStepResult { // to avoid raytracing twice (or sorting the results by depth), we raytrace -  add any enemy we find, and break only at a wall (so that we can render enemy in front of wall)
    block: Option<RaycastResult>,
    enemy: Option<RaycastResult>,
}
struct RaycastClosestCollisionPoint {
    distance_to_origin: f32,
    point: Vec2,
}
struct RaycastSystem;
impl RaycastSystem {
    fn raycast(
        origin: Vec2,
        player_angle: f32,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemies: &Enemies
    ) -> Vec<RaycastStepResult> {
        let mut res = Vec::new();
        for i in 0..AMOUNT_OF_RAYS {
            let ray_angle =
                player_angle +
                config::config::PLAYER_FOV / 2.0 -
                ((i as f32) / (AMOUNT_OF_RAYS as f32)) * config::config::PLAYER_FOV;

            let step_result = RaycastSystem::daa_raycast(origin, ray_angle, tile_map, enemies);
            res.push(step_result);
        }
        res
    }
    fn daa_raycast(
        origin: Vec2,
        specific_angle: f32,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemies: &Enemies
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
            curr_map_tile_x > 0 &&
            curr_map_tile_x < WORLD_WIDTH &&
            curr_map_tile_y > 0 &&
            curr_map_tile_y < WORLD_HEIGHT
        {
            // assume it hits a wall due to level design
            let is_x_side = dist_side_x < dist_side_y;
            if is_x_side {
                dist_side_x += relative_tile_dist_x;
                curr_map_tile_x = ((curr_map_tile_x as isize) + step_x) as usize;
            } else {
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
                        },
                    });
                    break;
                }
                EntityType::Enemy(id) => {
                    let enemy_pos = enemies.positions[id as usize];
                    let enemy_size = enemies.sizes[id as usize];
                    if
                        let Some(intersection) = Self::get_closest_collision_point(
                            origin,
                            direction,
                            enemy_pos,
                            enemy_size
                        )
                    {
                        if
                            intersection.distance_to_origin <
                            raycast_step_res.enemy.map_or(f32::INFINITY, |e| e.distance)
                        {
                            // we can see multiple enemies before a block, but we only care about closest, due to size of enemy sprite, rn (might change later)
                            raycast_step_res.enemy = Some(RaycastResult {
                                distance: intersection.distance_to_origin,
                                intersection_pos: intersection.point,
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
                }
                _ => {}
            }
        }
        raycast_step_res
    }
    fn get_closest_collision_point(
        origin: Vec2,
        direction: Vec2,
        enemy_pos: Vec2,
        enemy_size: Vec2
    ) -> Option<RaycastClosestCollisionPoint> {
        let t_near_x = (enemy_pos.x - origin.x) / direction.x;
        let t_near_y = (enemy_pos.y - origin.y) / direction.y;
        let t_far_x = (enemy_pos.x + enemy_size.x - origin.x) / direction.x;
        let t_far_y = (enemy_pos.y + enemy_size.y - origin.y) / direction.y;

        let (t_near_x, t_far_x) = if direction.x < 0.0 {
            (t_far_x, t_near_x)
        } else {
            (t_near_x, t_far_x)
        };

        let (t_near_y, t_far_y) = if direction.y < 0.0 {
            (t_far_y, t_near_y)
        } else {
            (t_near_y, t_far_y)
        };

        let t_near = t_near_x.max(t_near_y);
        let t_far = t_far_x.min(t_far_y);

        if t_near <= t_far && t_far > 0.0 {
            let intersection = origin + direction * t_near;
            const EPSILON: f32 = 0.1;
            if
                // might not be necessary
                intersection.x >= enemy_pos.x - EPSILON &&
                intersection.x <= enemy_pos.x + enemy_size.x + EPSILON &&
                intersection.y >= enemy_pos.y - EPSILON &&
                intersection.y <= enemy_pos.y + enemy_size.y + EPSILON
            {
                Some(RaycastClosestCollisionPoint {
                    distance_to_origin: t_near,
                    point: intersection,
                })
            } else {
                println!(
                    "Didnt hit the enemy {}, {}, {}, {}",
                    intersection.x >= enemy_pos.x - EPSILON,
                    intersection.x <= enemy_pos.x + enemy_size.x + EPSILON,
                    intersection.y >= enemy_pos.y - EPSILON,
                    intersection.y <= enemy_pos.y + EPSILON
                );
                None
            }
        } else {
            None
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
                    _ => {}
                }
            }
        }
    }
    fn render_player_and_enemies_on_map(player_pos: Vec2, enemies: &Enemies) {
        draw_rectangle(
            player_pos.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 + MAP_X_OFFSET,
            player_pos.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
            (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25,
            (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
            BLUE
        );
        for i in 0..enemies.positions.len() {
            let enemy_pos = &enemies.positions[i];
            let enemy_size = &enemies.sizes[i];
            let health = &enemies.healths[i];
            let x = enemy_pos.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25 + MAP_X_OFFSET;
            let y = enemy_pos.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25;
            draw_rectangle(
                x,
                y,
                enemy_size.x * (config::config::TILE_SIZE_X_PIXEL as f32) * 0.25,
                enemy_size.y * (config::config::TILE_SIZE_Y_PIXEL as f32) * 0.25,
                RED
            );
            let font_size = 16.0;
            draw_text(
                &format!("{}", health),
                x + enemy_size.x * 0.5 * (TILE_SIZE_X_PIXEL as f32) * 0.25 - font_size * 0.25,
                y + enemy_size.x * 0.5 * (TILE_SIZE_Y_PIXEL as f32) * 0.25,
                font_size,
                WHITE
            );
        }
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
        player_angle: f32,
        raycast_step_res: &Vec<RaycastStepResult>,
        enemy_animation_states: &Vec<AnimationState>,
        enemy_positions: &Vec<Vec2>,
        enemy_sizes: &Vec<Vec2>,
        world_layout: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) {
        let block_texture = TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::Stone).expect(
            "Stone texture failed to initialize"
        );
        let text_width = block_texture.width();
        let text_height = block_texture.height();

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
                let text_coord_x = if block.hit_from_x_side {
                    (block.intersection_pos.y * text_width) % text_width
                } else {
                    (block.intersection_pos.x * text_width) % text_width
                };
                draw_texture_ex(
                    block_texture,
                    (i as f32) * RAY_VERTICAL_STRIPE_WIDTH,
                    config::config::HALF_SCREEN_HEIGHT - wall_height / 2.0,
                    wall_color,
                    DrawTextureParams {
                        source: {
                            Some(Rect {
                                x: text_coord_x,
                                y: 0.0,
                                w: 1.0,
                                h: text_height,
                            })
                        },
                        dest_size: Some(Vec2::new(RAY_VERTICAL_STRIPE_WIDTH, wall_height)),
                        ..Default::default()
                    }
                );
            }
            if let Some(enemy) = &result.enemy {
                let enemy_handle = world_layout[enemy.map_idx.y as usize][enemy.map_idx.x as usize];
                let enemy_id = match enemy_handle {
                    EntityType::Enemy(idx) => idx,
                    _ => panic!("Invalid enemy handle"),
                };
                let enemy_size = enemy_sizes[enemy_id as usize];
                let enemy_center_pos = enemy_positions[enemy_id as usize] + enemy_size;
                let enemy_animation_state = &enemy_animation_states[enemy_id as usize];
                let enemy_texture_sheet = &enemy_animation_state.sprite_sheet;
                let aspect_ratio_sprite =
                    enemy_animation_state.spritesheet_offset_per_frame.x /
                    enemy_animation_state.spritesheet_offset_per_frame.y;
                let distance_vec = enemy_center_pos - player_origin;
                let sprite_height =
                    ((SCREEN_HEIGHT as f32) / (distance_vec.length() + 0.000001)).min(
                        SCREEN_HEIGHT as f32
                    ) / aspect_ratio_sprite;
                let sprite_screen_x = (i as f32) * RAY_VERTICAL_STRIPE_WIDTH;
                let dir_ray = enemy.intersection_pos.angle_between(player_origin);
                let dir_to_enemy = enemy_center_pos.angle_between(player_origin);
                let text_coord_x = ((dir_ray - dir_to_enemy) * text_width) / aspect_ratio_sprite;
                if
                    text_coord_x < 0.0 ||
                    text_coord_x >= enemy_animation_state.spritesheet_offset_per_frame.x
                {
                    continue;
                }
                let shade =
                    1.0 -
                    (distance_vec.length() / (WORLD_WIDTH.max(WORLD_HEIGHT) as f32)).clamp(
                        0.0,
                        1.0
                    );
                let sprite_source_and_color = enemy_animation_state.get_current_sprite_source();
                let sprite_color = Color::new(
                    sprite_source_and_color.color.r * shade,
                    sprite_source_and_color.color.g * shade,
                    sprite_source_and_color.color.b * shade,
                    1.0
                );

                let source_rect = Rect {
                    x: sprite_source_and_color.source.x + text_coord_x,
                    y: 0.0,
                    w: 1.0,
                    h: enemy_animation_state.spritesheet_offset_per_frame.y,
                };
                draw_texture_ex(
                    &enemy_texture_sheet,
                    sprite_screen_x,
                    config::config::HALF_SCREEN_HEIGHT - sprite_height / 2.0,
                    sprite_color,
                    DrawTextureParams {
                        source: Some(source_rect),
                        dest_size: Some(Vec2::new(RAY_VERTICAL_STRIPE_WIDTH, sprite_height)),
                        ..Default::default()
                    }
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
            positions: Vec::new(),
            velocities: Vec::new(),
            healths: Vec::new(),
            sizes: Vec::new(),
            animation_states: Vec::new(),
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
                            Vec2::new(1.0, -1.0),
                            1,
                            Vec2::new(1.0, 1.0),
                            AnimationState::default_skeleton()
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

    fn handle_game_event(&mut self, event: WorldEvent) {
        match event.event_type {
            WorldEventType::PlayerHitEnemy => {
                let enemy_handle: EntityType =
                    self.world_layout[event.target_tile_handle.y as usize]
                        [event.target_tile_handle.x as usize];
                match enemy_handle {
                    EntityType::Enemy(idx) => {
                        let health = self.enemies.healths
                            .get_mut(idx as usize)
                            .expect("Invalid handle in world layout");
                        if *health == 0 {
                            // avoid rescheduling animation callback
                            return;
                        }
                        *health -= 1;
                        if *health == 0 {
                            let enemy_animation_state =
                                &mut self.enemies.animation_states[idx as usize];
                            enemy_animation_state.set_callback(AnimationCallbackEvent {
                                event_type: AnimationCallbackEventType::KillEnemy,
                                target_handle: idx,
                            });
                            enemy_animation_state.set_physics_frames_per_update(20.0);
                            enemy_animation_state.color = Color::from_rgba(255, 0, 0, 255);
                        }
                    }
                    _ => panic!("Hit invalid enemy"),
                }
            }
            _ => panic!("Unahndled game event"),
        }
    }
    fn handle_input(&mut self) {
        if is_key_down(KeyCode::W) {
            self.player.vel = Vec2::new(self.player.angle.cos(), self.player.angle.sin()) * 2.0;
        } else if is_key_down(KeyCode::S) {
            self.player.vel = Vec2::new(-self.player.angle.cos(), -self.player.angle.sin()) * 2.0;
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
        MovementSystem::update_enemies(&mut self.enemies, &self.walls, &mut self.world_layout);
        let animation_callback_events = UpdateEnemyAnimation::update(
            self.player.pos,
            self.player.angle,
            &self.enemies.positions,
            &self.enemies.velocities,
            &mut self.enemies.animation_states
        );
        CallbackHandler::handle_animation_callbacks(
            animation_callback_events,
            &mut self.world_layout,
            &mut self.enemies
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
            &self.enemies
        );
        let end_time = get_time();
        let elapsed_time = end_time - start_time;
        RenderPlayerPOV::render_floor(
            &self.background_material,
            self.player.angle,
            player_ray_origin
        );
        RenderPlayerPOV::render_world(
            self.player.pos,
            self.player.angle,
            &raycast_result,
            &self.enemies.animation_states,
            &self.enemies.positions,
            &self.enemies.sizes,
            &self.world_layout
        );
        RenderPlayerPOV::render_weapon();
        RenderMap::render_world_layout(&self.world_layout);
        RenderMap::render_player_and_enemies_on_map(self.player.pos, &self.enemies);
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
