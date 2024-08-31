use core::panic;
use std::{ collections::{ HashMap }, f32::consts::PI, process::{ exit }, time::Duration };
use miniquad::{ BlendFactor, BlendState, BlendValue, Equation };
use ::rand::random;
use config::config::{
    AMOUNT_OF_RAYS,
    ENEMY_VIEW_DISTANCE,
    HALF_PLAYER_FOV,
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
use once_cell::sync::Lazy;
use macroquad::{ audio::{ load_sound, play_sound_once, Sound }, prelude::* };
use shaders::shaders::{
    DEFAULT_FRAGMENT_SHADER,
    DEFAULT_VERTEX_SHADER,
    FLOOR_FRAGMENT_SHADER,
    CAMERA_SHAKE_VERTEX_SHADER,
};
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EnemyHandle(pub u16);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WallHandle(pub u16);

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
    Wall(u16),
    None,
    Enemy(EnemyHandle),
}
enum WorldEventType {
    PlayerHitEnemy,
    EnemyHitPlayer,
}
#[derive(PartialEq, Clone, Copy, Eq, Hash)]
struct Tile {
    x: u16,
    y: u16,
}
impl Tile {
    fn from_vec2(pos: Vec2) -> Self {
        return Tile {
            x: pos.x.round() as u16,
            y: pos.y.round() as u16,
        };
    }
    fn from_u32(value: u32) -> Self {
        Tile {
            x: (value >> 16) as u16,
            y: (value & 0xffff) as u16,
        }
    }
}
struct WorldEventTileBased {
    event_type: WorldEventType,
    triggered_by_tile_handle: Tile,
    target_tile_handle: Tile,
}
impl WorldEventTileBased {
    fn player_hit_enemy(player_handle: Tile, enemy_handle: Tile) -> Self {
        return WorldEventTileBased {
            event_type: WorldEventType::PlayerHitEnemy,
            triggered_by_tile_handle: player_handle,
            target_tile_handle: enemy_handle,
        };
    }
    fn enemy_hit_player(player_handle: Tile, enemy_handle: Tile) -> Self {
        return WorldEventTileBased {
            event_type: WorldEventType::PlayerHitEnemy,
            triggered_by_tile_handle: enemy_handle,
            target_tile_handle: player_handle,
        };
    }
}
struct WorldEventHandleBased { // to avoid multiple tile lookups and inaccuracies due to rounding when intersecting for example
    event_type: WorldEventType,

    other_involved: u16,
}
impl WorldEventHandleBased {
    fn EnemyHitPlayer(enemy_handle: EnemyHandle) -> Self {
        WorldEventHandleBased {
            event_type: WorldEventType::EnemyHitPlayer,
            other_involved: enemy_handle.0,
        }
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
    target_handle: EnemyHandle,
}
impl AnimationCallbackEvent {
    fn none() -> Self {
        AnimationCallbackEvent {
            event_type: AnimationCallbackEventType::None,
            target_handle: EnemyHandle(0),
        }
    }
}
struct AnimationSprite {
    source: Rect, // what to sample from spritesheet
    color: Color,
}
#[derive(Clone)]
struct AnimationState {
    frame: u16,
    frames_amount: u16,
    spritesheet_offset_per_frame: Vec2,
    animation_type: EnemyAnimationType,
    sprite_sheet: Texture2D,
    color: Color,
    physics_frames_per_update: f32,
    elapsed_time: f32,
    flip_x: bool,
    callback_event: AnimationCallbackEvent,
}
impl AnimationState {
    fn default_skeleton() -> Self {
        let texture = TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::SkeletonFrontSpriteSheet).expect(
            "Failed to load Skeleton Front Spritesheet"
        );
        const FRAMES_AMOUNT: u16 = 3;
        let single_sprite_dimension_x = texture.width() / (FRAMES_AMOUNT as f32);
        AnimationState {
            frame: 0,
            frames_amount: FRAMES_AMOUNT,
            spritesheet_offset_per_frame: Vec2::new(single_sprite_dimension_x, 0.0),
            sprite_sheet: texture.clone(),
            color: WHITE,
            animation_type: EnemyAnimationType::SkeletonFront,
            physics_frames_per_update: 20.0 * PHYSICS_FRAME_TIME,
            elapsed_time: 0.0,
            flip_x: false,
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
    fn need_to_flip_x(&self) -> bool {
        match self.animation_type {
            EnemyAnimationType::SkeletonSide => self.flip_x,
            EnemyAnimationType::SkeletonBack => false,
            EnemyAnimationType::SkeletonFront => false,
        }
    }
    fn next(&mut self, dt: f32) -> AnimationCallbackEvent {
        assert!(self.physics_frames_per_update >= dt);
        self.elapsed_time += dt;
        let mut callback_event = AnimationCallbackEvent::none();
        if self.elapsed_time > self.physics_frames_per_update {
            if self.frame == self.frames_amount - 1 {
                callback_event = self.callback_event;
            }
            self.frame = (self.frame + 1) % self.frames_amount;
            self.elapsed_time = 0.0;
        }
        return callback_event;
    }
    fn change_animation(
        &mut self,
        new_spritesheet: Texture2D,
        new_animation_type: EnemyAnimationType,
        single_sprite_dimensions: Vec2
    ) {
        self.frame = 0;
        self.frames_amount = (new_spritesheet.width() / single_sprite_dimensions.x).trunc() as u16;
        let spritesheet_offset_per_frame_y = if
            new_spritesheet.height() < single_sprite_dimensions.y * 2.0
        {
            0.0
        } else {
            single_sprite_dimensions.y
        };
        self.spritesheet_offset_per_frame = Vec2::new(
            single_sprite_dimensions.x,
            spritesheet_offset_per_frame_y
        );
        self.sprite_sheet = new_spritesheet;
        self.animation_type = new_animation_type;
    }
}
#[derive(Clone, Copy, PartialEq, Debug)]
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
        aggressive_states: &Vec<bool>,
        velocities: &Vec<Vec2>,
        animation_states: &mut Vec<AnimationState>
    ) -> Vec<AnimationCallbackEvent> {
        let mut res: Vec<AnimationCallbackEvent> = Vec::new();
        for (((enemy_pos, velocity), is_aggressive), animation_state) in enemy_positions
            .iter()
            .zip(velocities.iter())
            .zip(aggressive_states.iter())
            .zip(animation_states.iter_mut()) {
            let callback_event = animation_state.next(PHYSICS_FRAME_TIME);
            res.push(callback_event);

            if *is_aggressive {
                if animation_state.animation_type != EnemyAnimationType::SkeletonFront {
                    animation_state.change_animation(
                        TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::SkeletonFrontSpriteSheet)
                            .expect("Failed to load spritesheet skeleton")
                            .clone(),
                        EnemyAnimationType::SkeletonFront,
                        Vec2::new(31.0, 48.0)
                    );
                }
                continue;
            }
            let to_player = player_origin - *enemy_pos;
            let vel_enemy_rel_player = velocity.angle_between(to_player);
            match vel_enemy_rel_player {
                angle if angle > 0.0 && angle < std::f32::consts::FRAC_PI_4 => {
                    if animation_state.animation_type != EnemyAnimationType::SkeletonSide {
                        animation_state.change_animation(
                            TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::SkeletonSideSpriteSheet)
                                .expect("Failed to load spritesheet skeleton")
                                .clone(),
                            EnemyAnimationType::SkeletonSide,
                            Vec2::new(31.0, 48.0)
                        );
                    }
                    animation_state.flip_x = true;
                }
                angle if angle <= 0.0 && angle > -PI => {
                    if animation_state.animation_type != EnemyAnimationType::SkeletonSide {
                        animation_state.change_animation(
                            TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::SkeletonSideSpriteSheet)
                                .expect("Failed to load spritesheet skeleton")
                                .clone(),
                            EnemyAnimationType::SkeletonSide,
                            Vec2::new(31.0, 48.0)
                        );
                    }
                    animation_state.flip_x = false;
                }
                angle if
                    (angle > 0.0 && angle > std::f32::consts::FRAC_2_PI) ||
                    (angle < 0.0 && angle > -std::f32::consts::FRAC_2_PI)
                => {
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
            }
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
                    let mut enemy_information = enemies.get_enemy_information(enemy_idx.0);
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
                    enemies.destroy_enemy(enemy_idx.0);
                }
                AnimationCallbackEventType::None => {}
            }
        }
    }
}

struct CollisionData {
    x_collisions: Vec<u32>,
    y_collisions: Vec<u32>,
    collision_times: Vec<Duration>,
}

impl CollisionData {
    fn new(enemy_count: usize) -> Self {
        CollisionData {
            x_collisions: vec![0; enemy_count],
            y_collisions: vec![0; enemy_count],
            collision_times: vec![Duration::from_secs(0); enemy_count],
        }
    }
}
struct EnemyInformation {
    idx: u16,
    pos: Vec2,
    vel: Vec2,
    health: u8,
    size: Vec2,
    animation_state: AnimationState,
    aggressive: bool,
}
struct Enemies {
    positions: Vec<Vec2>,
    velocities: Vec<Vec2>,
    healths: Vec<u8>,
    sizes: Vec<Vec2>,
    animation_states: Vec<AnimationState>,
    aggressive_states: Vec<bool>,
    collision_data: CollisionData,
}

impl Enemies {
    fn new() -> Self {
        Enemies {
            positions: Vec::new(),
            velocities: Vec::new(),
            healths: Vec::new(),
            sizes: Vec::new(),
            animation_states: Vec::new(),
            collision_data: CollisionData::new(0),
            aggressive_states: Vec::new(),
        }
    }

    fn new_enemy(
        &mut self,
        pos: Vec2,
        velocity: Vec2,
        health: u8,
        size: Vec2,
        animation: AnimationState
    ) -> usize {
        let index = self.positions.len();
        self.positions.push(pos);
        self.velocities.push(velocity);
        self.healths.push(health);
        self.sizes.push(size);
        self.animation_states.push(animation);
        self.collision_data.x_collisions.push(0);
        self.collision_data.y_collisions.push(0);
        self.collision_data.collision_times.push(Duration::from_secs(0));
        self.aggressive_states.push(false);
        index
    }
    fn destroy_enemy(&mut self, idx: u16) {
        self.positions.swap_remove(idx as usize);
        self.velocities.swap_remove(idx as usize);
        self.healths.swap_remove(idx as usize);
        self.sizes.swap_remove(idx as usize);
        self.animation_states.swap_remove(idx as usize);
        self.collision_data.x_collisions.swap_remove(idx as usize);
        self.collision_data.y_collisions.swap_remove(idx as usize);
        self.collision_data.collision_times.swap_remove(idx as usize);
        self.aggressive_states.swap_remove(idx as usize);
    }
    fn get_enemy_information(&self, idx: u16) -> EnemyInformation {
        let idx = idx as usize;
        EnemyInformation {
            idx: idx as u16,
            pos: *self.positions.get(idx).expect("Tried to acccess invalid enemy idx"),
            vel: *self.velocities.get(idx).expect("Tried to acccess invalid enemy idx"),
            health: *self.healths.get(idx).expect("Tried to acccess invalid enemy idx"),
            size: *self.sizes.get(idx).expect("Tried to acccess invalid enemy idx"),
            animation_state: self.animation_states
                .get(idx)
                .expect("Tried to acccess invalid enemy idx")
                .clone(),
            aggressive: *self.aggressive_states
                .get(idx)
                .expect("Tried to acccess invalid enemy idx"),
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
        *self.aggressive_states.get_mut(idx).expect("Invalid enemy information update") =
            enemy_information.aggressive;
    }
}
struct Weapon {
    reload_frames_t: u8, // in physics frames
    damage: u8,
    range: u8,
    elapsed_reload_t: u8,
}
impl Weapon {
    fn default() -> Self {
        Weapon {
            reload_frames_t: 20,
            damage: 1,
            range: 8,
            elapsed_reload_t: 0,
        }
    }
}
struct WeaponSystem;
impl WeaponSystem {
    fn update_reload(player_weapon: &mut Weapon) {
        if player_weapon.elapsed_reload_t > 0 {
            player_weapon.elapsed_reload_t += 1;
        }
        if player_weapon.elapsed_reload_t >= player_weapon.reload_frames_t {
            player_weapon.elapsed_reload_t = 0;
        }
    }
}
struct ShootEvent {
    world_event: Option<WorldEventTileBased>,
    still_reloading: bool,
}
struct Player {
    pos: Vec2,
    angle: f32,
    vel: Vec2,
    health: u16,
    weapon: Weapon,
}
impl Player {
    fn shoot(
        &mut self,
        world_layout: [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemies: &Enemies
    ) -> ShootEvent {
        const RAY_SPREAD: f32 = PLAYER_FOV / 2.0 / 10.0; // basically defines the hitbox of the player shooting
        let angles = [self.angle - RAY_SPREAD, self.angle, self.angle + RAY_SPREAD];
        if self.weapon.elapsed_reload_t > 0 {
            return ShootEvent {
                world_event: None,
                still_reloading: true,
            };
        }
        self.weapon.elapsed_reload_t = 1; // start reloading
        for &angle in &angles {
            let hit_enemy = RaycastSystem::shoot_bullet_raycast(self.pos, angle, &world_layout);
            match hit_enemy {
                Some(enemy) => {
                    let enemy_pos = enemies.positions
                        .get(enemy.0 as usize)
                        .expect("Invalid enemy handle");
                    let enemy_dist = self.pos.distance(*enemy_pos);
                    let event = if (enemy_dist.round() as u32) > (self.weapon.range as u32) {
                        None
                    } else {
                        Some(
                            WorldEventTileBased::player_hit_enemy(
                                Tile::from_vec2(self.pos),
                                Tile::from_vec2(*enemy_pos)
                            )
                        )
                    };
                    return ShootEvent {
                        world_event: event,
                        still_reloading: false,
                    };
                }
                _ => {}
            }
        }
        return ShootEvent {
            world_event: None,
            still_reloading: false,
        };
    }
}
struct MovingEntityCollisionSystem;

impl MovingEntityCollisionSystem {
    fn check_player_enemy_collisions(
        player_pos: &Vec2,
        world_layout: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemy_positions: &Vec<Vec2>,
        enemy_sizes: &Vec<Vec2>
    ) -> Option<WorldEventHandleBased> {
        let player_size = Vec2::new(1.0, 1.0);
        let check_radius = 2; // Adjust this value based on the maximum enemy size

        let start_x = ((player_pos.x as i32) - check_radius).max(0) as usize;
        let end_x = ((player_pos.x as i32) + check_radius + 1).min(WORLD_WIDTH as i32) as usize;
        let start_y = ((player_pos.y as i32) - check_radius).max(0) as usize;
        let end_y = ((player_pos.y as i32) + check_radius + 1).min(WORLD_HEIGHT as i32) as usize;

        for y in start_y..end_y {
            for x in start_x..end_x {
                if let EntityType::Enemy(enemy_handle) = world_layout[y][x] {
                    let enemy_index = enemy_handle.0 as usize;
                    let enemy_pos = &enemy_positions[enemy_index];
                    let enemy_size = &enemy_sizes[enemy_index];

                    if Self::check_collision(player_pos, &player_size, enemy_pos, enemy_size) {
                        return Some(WorldEventHandleBased::EnemyHitPlayer(enemy_handle));
                    }
                }
            }
        }

        None
    }

    fn check_enemy_enemy_collisions(
        world_layout: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        enemy_positions: &Vec<Vec2>,
        enemy_sizes: &Vec<Vec2>
    ) -> Vec<(EnemyHandle, EnemyHandle)> {
        let mut collisions = Vec::new();
        let check_radius = 2; // Adjust this value based on the maximum enemy size

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if let EntityType::Enemy(enemy_handle1) = world_layout[y][x] {
                    let enemy_index1 = enemy_handle1.0 as usize;
                    let enemy_pos1 = &enemy_positions[enemy_index1];
                    let enemy_size1 = &enemy_sizes[enemy_index1];

                    let start_x = ((enemy_pos1.x as i32) - check_radius).max(0) as usize;
                    let end_x = ((enemy_pos1.x as i32) + check_radius + 1).min(
                        WORLD_WIDTH as i32
                    ) as usize;
                    let start_y = ((enemy_pos1.y as i32) - check_radius).max(0) as usize;
                    let end_y = ((enemy_pos1.y as i32) + check_radius + 1).min(
                        WORLD_HEIGHT as i32
                    ) as usize;

                    for check_y in start_y..end_y {
                        for check_x in start_x..end_x {
                            if
                                let EntityType::Enemy(enemy_handle2) =
                                    world_layout[check_y][check_x]
                            {
                                if enemy_handle1 != enemy_handle2 {
                                    let enemy_index2 = enemy_handle2.0 as usize;
                                    let enemy_pos2 = &enemy_positions[enemy_index2];
                                    let enemy_size2 = &enemy_sizes[enemy_index2];

                                    if
                                        Self::check_collision(
                                            enemy_pos1,
                                            enemy_size1,
                                            enemy_pos2,
                                            enemy_size2
                                        )
                                    {
                                        collisions.push((enemy_handle1, enemy_handle2));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        collisions
    }

    fn check_collision(pos1: &Vec2, size1: &Vec2, pos2: &Vec2, size2: &Vec2) -> bool {
        let center1 = Vec2::new(pos1.x + size1.x / 2.0, pos1.y + size1.y / 2.0);
        let center2 = Vec2::new(pos2.x + size2.x / 2.0, pos2.y + size2.y / 2.0);

        let distance_x = (center1.x - center2.x).abs();
        let distance_y = (center1.y - center2.y).abs();

        let min_distance_x = (size1.x + size2.x) / 2.0;
        let min_distance_y = (size1.y + size2.y) / 2.0;

        distance_x < min_distance_x && distance_y < min_distance_y
    }
}
struct MovementSystem;
impl MovementSystem {
    fn update_enemies(
        enemies: &mut Enemies,
        walls: &Vec<Vec2>,
        world_layout: &mut [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
        current_time: Duration
    ) {
        const COLLISION_THRESHOLD: u32 = 5;
        const COLLISION_TIME_WINDOW: Duration = Duration::from_secs(2);

        for (id, ((pos, vel), size)) in enemies.positions
            .iter_mut()
            .zip(enemies.velocities.iter_mut())
            .zip(enemies.sizes.iter())
            .enumerate() {
            let prev_tiles = Self::get_occupied_tiles(*pos, *size);
            let mut new_pos = *pos + *vel * PHYSICS_FRAME_TIME;

            let (collided_x, collided_y) = Self::resolve_wall_collisions(&mut new_pos, walls, *pos);

            if collided_x {
                enemies.collision_data.x_collisions[id] += 1;
            }
            if collided_y {
                enemies.collision_data.y_collisions[id] += 1;
            }

            if collided_x || collided_y {
                enemies.collision_data.collision_times[id] = current_time;
            }

            let time_since_last_collision =
                current_time - enemies.collision_data.collision_times[id];

            if time_since_last_collision <= COLLISION_TIME_WINDOW {
                if enemies.collision_data.x_collisions[id] >= COLLISION_THRESHOLD {
                    vel.x *= -1.0;
                    enemies.collision_data.x_collisions[id] = 0;
                }
                if enemies.collision_data.y_collisions[id] >= COLLISION_THRESHOLD {
                    vel.y *= -1.0;
                    enemies.collision_data.y_collisions[id] = 0;
                }
            } else {
                enemies.collision_data.x_collisions[id] = 0;
                enemies.collision_data.y_collisions[id] = 0;
            }

            *pos = new_pos;

            let new_tiles = Self::get_occupied_tiles(*pos, *size);
            for tile in prev_tiles {
                match world_layout[tile.y as usize][tile.x as usize] {
                    EntityType::Enemy(handle) => {
                        if (handle.0 as usize) != id {
                            continue;
                        }
                        world_layout[tile.y as usize][tile.x as usize] = EntityType::None;
                    }
                    _ => {}
                }
            }
            for tile in new_tiles {
                match world_layout[tile.y as usize][tile.x as usize] {
                    EntityType::None => {
                        world_layout[tile.y as usize][tile.x as usize] = EntityType::Enemy(
                            EnemyHandle(id as u16)
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    fn resolve_wall_collisions(
        position: &mut Vec2,
        walls: &Vec<Vec2>,
        old_position: Vec2
    ) -> (bool, bool) {
        let mut collided_x = false;
        let mut collided_y = false;

        for wall in walls.iter() {
            let point_1 = Vec2::new(wall.x + 0.5, wall.y + 0.5);
            let point_2 = Vec2::new(position.x + 0.5, position.y + 0.5);

            let distance_x = (point_2.x - point_1.x).abs();
            let distance_y = (point_2.y - point_1.y).abs();

            if distance_x < 1.0 && distance_y < 1.0 {
                if distance_x > distance_y {
                    position.x = old_position.x;
                    collided_x = true;
                } else {
                    position.y = old_position.y;
                    collided_y = true;
                }
            }
        }

        (collided_x, collided_y)
    }

    fn get_occupied_tiles(pos: Vec2, size: Vec2) -> Vec<Tile> {
        let mut tiles = Vec::new();
        let start_x = pos.x.floor() as u16;
        let start_y = pos.y.floor() as u16;
        let end_x = (pos.x + size.x - 0.01).floor() as u16;
        let end_y = (pos.y + size.y - 0.01).floor() as u16;

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
        Self::player_resolve_wall_collisions(&mut player.pos, walls);
        let new_tile = Tile::from_vec2(player.pos);
        world_layout[new_tile.y as usize][new_tile.x as usize] = EntityType::Player;
        if prev_tile != new_tile {
            assert!(world_layout[prev_tile.y as usize][prev_tile.x as usize] == EntityType::Player);
            world_layout[prev_tile.y as usize][prev_tile.x as usize] = EntityType::None;
        }
    }

    fn player_resolve_wall_collisions(position: &mut Vec2, walls: &Vec<Vec2>) {
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
    ) -> Vec<RaycastStepResult> {
        let mut res = Vec::new();
        for i in 0..AMOUNT_OF_RAYS {
            let ray_angle =
                player_angle +
                config::config::PLAYER_FOV / 2.0 -
                ((i as f32) / (AMOUNT_OF_RAYS as f32)) * config::config::PLAYER_FOV;

            let step_result = RaycastSystem::daa_raycast(origin, ray_angle, tile_map);
            if let Some(step) = step_result {
                res.push(step);
            }
        }
        res
    }

    fn daa_raycast(
        origin: Vec2,
        specific_angle: f32,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) -> Option<RaycastStepResult> {
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
            let is_x_side = dist_side_x < dist_side_y;
            if is_x_side {
                dist_side_x += relative_tile_dist_x;
                curr_map_tile_x = ((curr_map_tile_x as isize) + step_x) as usize;
            } else {
                dist_side_y += relative_tile_dist_y;
                curr_map_tile_y = ((curr_map_tile_y as isize) + step_y) as usize;
            }
            match tile_map[curr_map_tile_y][curr_map_tile_x] {
                EntityType::Wall(_) => {
                    let distance = if is_x_side {
                        dist_side_x - relative_tile_dist_x
                    } else {
                        dist_side_y - relative_tile_dist_y
                    };
                    return Some(RaycastStepResult {
                        intersection_pos: Vec2::new(
                            origin.x + direction.x * distance,
                            origin.y + direction.y * distance
                        ),
                        intersection_site: if is_x_side {
                            if direction.x > 0.0 {
                                IntersectedSite::XLeft
                            } else {
                                IntersectedSite::XRight
                            }
                        } else {
                            if direction.y > 0.0 {
                                IntersectedSite::YTop
                            } else {
                                IntersectedSite::YBottom
                            }
                        },
                        corrected_distance: if is_x_side {
                            dist_side_x - relative_tile_dist_x
                        } else {
                            dist_side_y - relative_tile_dist_y
                        },
                    });
                }
                _ => {}
            }
        }
        return None;
    }
    fn shoot_bullet_raycast(
        origin: Vec2,
        specific_angle: f32,
        tile_map: &[[EntityType; WORLD_WIDTH]; WORLD_HEIGHT]
    ) -> Option<EnemyHandle> {
        // NOTE returns a handle
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
            let is_x_side = dist_side_x < dist_side_y;
            if is_x_side {
                dist_side_x += relative_tile_dist_x;
                curr_map_tile_x = ((curr_map_tile_x as isize) + step_x) as usize;
            } else {
                dist_side_y += relative_tile_dist_y;
                curr_map_tile_y = ((curr_map_tile_y as isize) + step_y) as usize;
            }
            match tile_map[curr_map_tile_y][curr_map_tile_x] {
                EntityType::Wall(_) => {
                    return None;
                }
                EntityType::Enemy(handle) => {
                    return Some(handle);
                }
                _ => {}
            }
        }
        None
    }
}
struct RenderMap;
impl RenderMap {
    #[inline(always)]
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
    #[inline(always)]
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
    #[inline(always)]
    fn render_rays(player_origin: Vec2, raycast_result: &Vec<RaycastStepResult>) {
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
        }
    }
}
struct RenderPlayerPOV;
impl RenderPlayerPOV {
    #[inline(always)]
    fn render_floor(material: &Material, player_angle: f32, player_pos: Vec2) {
        let left_most_ray_dir = Vec2::new(
            (player_angle + HALF_PLAYER_FOV).cos(),
            (player_angle + HALF_PLAYER_FOV).sin()
        );
        let right_most_ray_dir = Vec2::new(
            (player_angle - HALF_PLAYER_FOV).cos(),
            (player_angle - HALF_PLAYER_FOV).sin()
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
    #[inline(always)]
    fn render_walls(
        raycast_step_res: &Vec<RaycastStepResult>,
        z_buffer: &mut [f32; AMOUNT_OF_RAYS]
    ) {
        let block_texture = TEXTURE_TYPE_TO_TEXTURE2D.get(&Textures::Stone).expect(
            "Stone texture failed to initialize"
        );
        let text_width = block_texture.width();
        let text_height = block_texture.height();

        for (i, result) in raycast_step_res.iter().enumerate() {
            let distance = result.corrected_distance;
            z_buffer[i] = distance;
            let wall_color = GREEN;
            let wall_height = ((SCREEN_HEIGHT as f32) / (distance - 0.5 + 0.000001)).min(
                SCREEN_HEIGHT as f32
            );
            let shade = 1.0 - (distance / (WORLD_WIDTH.min(WORLD_HEIGHT) as f32)).clamp(0.0, 1.0);
            let wall_color = Color::new(
                wall_color.r * shade,
                wall_color.g * shade,
                wall_color.b * shade,
                1.0
            );
            let is_x_side =
                result.intersection_site == IntersectedSite::XLeft ||
                result.intersection_site == IntersectedSite::XRight;
            let wall_color = if is_x_side {
                wall_color
            } else {
                Color::new(wall_color.r * 0.8, wall_color.g * 0.8, wall_color.b * 0.8, 1.0)
            };
            let text_coord_x = if is_x_side {
                (result.intersection_pos.y * text_width) % text_width
            } else {
                (result.intersection_pos.x * text_width) % text_width
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
    }
    #[inline(always)]
    fn render_enemies(
        z_buffer: &[f32; AMOUNT_OF_RAYS],
        player_pos: Vec2,
        enemies: &Vec<SeenEnemy>,
        positions: &Vec<Vec2>,
        animation_states: &Vec<AnimationState>
    ) {
        for enemy in enemies {
            let rel_sprite_x = (enemy.relative_angle - HALF_PLAYER_FOV).abs() / (PI / 2.0);
            let sprite_x = rel_sprite_x * (SCREEN_WIDTH as f32);
            let animation = &animation_states[enemy.enemy_handle.0 as usize];
            let distance_to_player: f32 =
                player_pos.distance(positions[enemy.enemy_handle.0 as usize]) + 0.0001;
            let sprite_height = ((SCREEN_HEIGHT as f32) / distance_to_player - 0.5).min(
                SCREEN_HEIGHT as f32
            );
            let screen_y = HALF_SCREEN_HEIGHT - sprite_height / 2.0;
            let texture_width = animation.spritesheet_offset_per_frame.x;
            let growth_factor = sprite_height / animation.sprite_sheet.height();
            let aspect_ratio =
                animation.spritesheet_offset_per_frame.x / animation.sprite_sheet.height();
            let shade =
                1.0 - (distance_to_player / (WORLD_WIDTH.min(WORLD_HEIGHT) as f32)).clamp(0.0, 1.0);
            let color = Color::new(
                animation.color.r * shade,
                animation.color.g * shade,
                animation.color.b * shade,
                1.0
            );
            let curr_animation_text_coord_x =
                animation.spritesheet_offset_per_frame.x * (animation.frame as f32);

            let x_range: Box<dyn Iterator<Item = usize>> = if animation.need_to_flip_x() {
                Box::new((0..texture_width as usize).rev())
            } else {
                Box::new(0..texture_width as usize)
            };

            for x in x_range {
                let screen_x = sprite_x + (x as f32) * growth_factor * aspect_ratio;
                if
                    screen_x >= (SCREEN_WIDTH as f32) ||
                    z_buffer[screen_x as usize] < distance_to_player
                {
                    continue;
                }
                let source_x = if animation.need_to_flip_x() {
                    curr_animation_text_coord_x + (texture_width - 1.0 - (x as f32))
                } else {
                    curr_animation_text_coord_x + (x as f32)
                };
                let source_rect = Rect {
                    x: source_x,
                    y: 0.0,
                    w: 1.0,
                    h: animation.sprite_sheet.height(),
                };
                draw_texture_ex(
                    &animation.sprite_sheet,
                    screen_x,
                    screen_y,
                    color,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(growth_factor * aspect_ratio, sprite_height)),
                        source: Some(source_rect),
                        ..Default::default()
                    }
                );
            }
        }
    }
    #[inline(always)]
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
#[derive(Clone, Copy, PartialEq)]
enum IntersectedSite {
    XLeft,
    XRight,
    YTop,
    YBottom,
}
#[derive(Clone, Copy)]
struct RaycastStepResult {
    intersection_site: IntersectedSite,
    intersection_pos: Vec2,
    corrected_distance: f32,
}
struct SeenEnemy {
    enemy_handle: EnemyHandle,
    relative_angle: f32,
}
struct EnemyAggressionSystem;
impl EnemyAggressionSystem {
    fn toggle_enemy_aggressive(
        player_pos: Vec2,
        enemy_positions: &Vec<Vec2>,
        enemy_velocities: &mut Vec<Vec2>,
        aggressive_states: &mut Vec<bool>
    ) {
        let tile_pos_player = player_pos.trunc();
        for ((enemy_pos, enemy_vel), is_aggressive) in enemy_positions
            .iter()
            .zip(enemy_velocities.iter_mut())
            .zip(aggressive_states.iter_mut()) {
            let dist_vector = tile_pos_player - enemy_pos.trunc();
            if dist_vector.length() <= ENEMY_VIEW_DISTANCE {
                *is_aggressive = true;
                *enemy_vel = dist_vector.normalize();
            } else if *is_aggressive {
                *is_aggressive = false;
                *enemy_vel = Vec2::new(1.0, -1.0);
            }
        }
    }
}
struct CameraShake {
    duration: f32,
    intensity: f32,
    current_time: f32,
}

impl CameraShake {
    fn new(duration: f32, intensity: f32) -> Self {
        Self {
            duration,
            intensity,
            current_time: 0.0,
        }
    }

    fn update(&mut self, dt: f32) -> Vec2 {
        if self.current_time >= self.duration {
            return Vec2::ZERO;
        }
        self.current_time += dt;
        let progress = self.current_time / self.duration;
        let damping = 1.0 - progress;

        let angle = random::<f32>() * std::f32::consts::TAU;
        let offset = Vec2::new(angle.cos(), angle.sin()) * self.intensity * damping;
        offset
    }
}
enum VisualEffect {
    CameraShake(CameraShake),
    BloodyScreen,
    None,
}
struct World {
    world_layout: [[EntityType; WORLD_WIDTH]; WORLD_HEIGHT],
    background_material: Material,
    camera_shake_material: Material,
    shoot_sound: Sound,
    reload_sound: Sound,
    walls: Vec<Vec2>,
    enemies: Enemies,
    player: Player,
    postprocessing: VisualEffect,
}
impl World {
    async fn default() -> Self {
        let mut walls = Vec::new();
        let mut enemies = Enemies::new();
        let mut player = Player {
            pos: Vec2::new(0.0, 0.0),
            angle: 0.0,
            vel: Vec2::new(0.0, 0.0),
            health: 3,
            weapon: Weapon::default(),
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
                        world_layout[y][x] = EntityType::Wall(walls.len() as u16);
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
                        world_layout[y][x] = EntityType::Enemy(EnemyHandle(handle as u16));
                    }
                    _ => panic!("Invalid entity type in world layout"),
                };
            }
        }

        let background_material = load_material(
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
        let camera_shake_material = load_material(
            ShaderSource::Glsl {
                vertex: &CAMERA_SHAKE_VERTEX_SHADER,
                fragment: &DEFAULT_FRAGMENT_SHADER,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc {
                        name: "screen_size".to_string(),
                        uniform_type: UniformType::Float2,
                        array_count: 1,
                    },
                    UniformDesc {
                        name: "shake_offset".to_string(),
                        uniform_type: UniformType::Float2,
                        array_count: 1,
                    }
                ],
                pipeline_params: PipelineParams {
                    color_blend: Some(
                        BlendState::new(
                            Equation::Add,
                            BlendFactor::Value(BlendValue::SourceAlpha),
                            BlendFactor::OneMinusValue(BlendValue::SourceAlpha)
                        )
                    ),
                    alpha_blend: Some(
                        BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One)
                    ),
                    ..Default::default()
                },
                ..Default::default()
            }
        ).unwrap();
        let shoot_sound = load_sound("sounds/pistol_shoot.wav").await.unwrap();
        let reload_sound = load_sound("sounds/reload.wav").await.unwrap();
        Self {
            world_layout,
            background_material: background_material,
            camera_shake_material: camera_shake_material,
            walls,
            enemies,
            player,
            shoot_sound,
            reload_sound,
            postprocessing: VisualEffect::None,
        }
    }

    fn handle_world_event_tile_based(&mut self, event: WorldEventTileBased) {
        match event.event_type {
            WorldEventType::PlayerHitEnemy => {
                let enemy_handle: EntityType =
                    self.world_layout[event.target_tile_handle.y as usize]
                        [event.target_tile_handle.x as usize];
                match enemy_handle {
                    EntityType::Enemy(idx) => {
                        let health = self.enemies.healths
                            .get_mut(idx.0 as usize)
                            .expect("Invalid handle in world layout");
                        if *health == 0 {
                            // avoid rescheduling animation callback
                            return;
                        }

                        if *health <= self.player.weapon.damage {
                            let enemy_animation_state =
                                &mut self.enemies.animation_states[idx.0 as usize];
                            enemy_animation_state.set_callback(AnimationCallbackEvent {
                                event_type: AnimationCallbackEventType::KillEnemy,
                                target_handle: idx,
                            });
                            enemy_animation_state.set_physics_frames_per_update(20.0);
                            enemy_animation_state.color = Color::from_rgba(255, 0, 0, 255);
                            return;
                        }

                        *health -= self.player.weapon.damage;
                    }
                    _ => panic!("Hit invalid enemy"),
                }
            }
            WorldEventType::EnemyHitPlayer => {
                let enemy_handle: EntityType =
                    self.world_layout[event.triggered_by_tile_handle.y as usize]
                        [event.triggered_by_tile_handle.x as usize];
                match enemy_handle {
                    EntityType::Enemy(idx) => {
                        self.player.pos =
                            self.player.pos + self.enemies.velocities[idx.0 as usize] * 5.0; // move player away
                        if self.player.health == 1 {
                            exit(1);
                        }
                        self.player.health -= 1;
                        self.postprocessing = VisualEffect::CameraShake(
                            CameraShake::new(0.5, 10.0)
                        );
                    }
                    _ => panic!("Hit invalid enemy"),
                }
            }
        }
    }
    fn move_player(&mut self, delta: Vec2) {
        let old_pos = self.player.pos;

        self.player.pos += delta;

        let old_tile_x = old_pos.x.floor() as usize;
        let old_tile_y = old_pos.y.floor() as usize;
        let new_tile_x = self.player.pos.x.floor() as usize;
        let new_tile_y = self.player.pos.y.floor() as usize;

        if old_tile_x != new_tile_x || old_tile_y != new_tile_y {
            if self.world_layout[old_tile_y][old_tile_x] == EntityType::Player {
                self.world_layout[old_tile_y][old_tile_x] = EntityType::None;
            }
            self.world_layout[new_tile_y][new_tile_x] = EntityType::Player;
        }
    }
    fn handle_world_event_handle_based(&mut self, event: WorldEventHandleBased) {
        match event.event_type {
            WorldEventType::EnemyHitPlayer => {
                self.move_player(self.enemies.velocities[event.other_involved as usize] * 0.5); // move player away
                if self.player.health == 1 {
                    exit(1);
                }
                self.player.health -= 1;
                self.postprocessing = VisualEffect::CameraShake(CameraShake::new(0.5, 10.0));
            }
            WorldEventType::PlayerHitEnemy => todo!(),
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
            self.player.angle = self.player.angle.rem_euclid(2.0 * PI);
        }
        if is_key_down(KeyCode::D) {
            self.player.angle += 0.75 * get_frame_time();
            self.player.angle = self.player.angle.rem_euclid(2.0 * PI);
        }
        if is_key_pressed(KeyCode::Space) {
            let shoot_event = self.player.shoot(self.world_layout, &self.enemies);
            if shoot_event.still_reloading {
                play_sound_once(&self.reload_sound);
            } else {
                play_sound_once(&self.shoot_sound);
            }
            if let Some(event) = shoot_event.world_event {
                self.handle_world_event_tile_based(event);
            }
        }
    }

    fn update(&mut self) {
        assert!(self.enemies.positions.len() < 65536);
        assert!(self.world_layout.len() < 65536 && self.world_layout[0].len() < 65536);
        assert!(self.walls.len() < 65536);
        WeaponSystem::update_reload(&mut self.player.weapon);
        MovementSystem::update_player(&mut self.player, &self.walls, &mut self.world_layout);
        MovementSystem::update_enemies(
            &mut self.enemies,
            &self.walls,
            &mut self.world_layout,
            Duration::from_secs_f32(get_time() as f32)
        );
        let event = MovingEntityCollisionSystem::check_player_enemy_collisions(
            &self.player.pos,
            &self.world_layout,
            &self.enemies.positions,
            &self.enemies.sizes
        );
        if let Some(event) = event {
            self.handle_world_event_handle_based(event);
        }
        EnemyAggressionSystem::toggle_enemy_aggressive(
            self.player.pos,
            &self.enemies.positions,
            &mut self.enemies.velocities,
            &mut self.enemies.aggressive_states
        );
        // we can rewrite the rendering logic to use this, then put the callbacks into a queue and only update visible enemies animations
        let animation_callback_events = UpdateEnemyAnimation::update(
            self.player.pos,
            self.player.angle,
            &self.enemies.positions,
            &self.enemies.aggressive_states,
            &self.enemies.velocities,
            &mut self.enemies.animation_states
        );
        CallbackHandler::handle_animation_callbacks(
            animation_callback_events,
            &mut self.world_layout,
            &mut self.enemies
        );
    }

    fn draw(&mut self) {
        clear_background(LIGHTGRAY);
        let player_ray_origin = self.player.pos + Vec2::new(0.5, 0.5);
        let start_time: f64 = get_time();
        let raycast_result = RaycastSystem::raycast(
            player_ray_origin,
            self.player.angle,
            &self.world_layout
        );
        let end_time = get_time();
        let elapsed_time = end_time - start_time;
        RenderPlayerPOV::render_floor(
            &self.background_material,
            self.player.angle,
            player_ray_origin
        );
        match &mut self.postprocessing {
            VisualEffect::CameraShake(shake) => {
                gl_use_material(&self.camera_shake_material);
                let shake_offset = shake.update(get_frame_time());
                self.camera_shake_material.set_uniform(
                    "screen_size",
                    Vec2::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32)
                );
                self.camera_shake_material.set_uniform("shake_offset", shake_offset);
                if shake_offset == Vec2::ZERO {
                    self.postprocessing = VisualEffect::None;
                }
            }
            VisualEffect::None => {}
            _ => todo!(),
        }
        let mut z_buffer = [f32::MAX; AMOUNT_OF_RAYS as usize];
        RenderPlayerPOV::render_walls(&raycast_result, &mut z_buffer);
        let mut seen_enemies = Vec::new();
        for row in 0..self.world_layout.len() {
            for entity in self.world_layout[row] {
                match entity {
                    EntityType::Enemy(enemy_handle) => {
                        if (enemy_handle.0 as usize) > self.enemies.positions.len() - 1 {
                            continue;
                        }
                        let enemy_pos = self.enemies.positions[enemy_handle.0 as usize];
                        let angle_to_enemy = (enemy_pos.y - self.player.pos.y).atan2(
                            enemy_pos.x - self.player.pos.x
                        );
                        let normalized_angle_to_enemy =
                            (angle_to_enemy + 2.0 * std::f32::consts::PI) %
                            (2.0 * std::f32::consts::PI);
                        let mut angle_diff = normalized_angle_to_enemy - self.player.angle;
                        if angle_diff > std::f32::consts::PI {
                            angle_diff -= 2.0 * std::f32::consts::PI;
                        } else if angle_diff < -std::f32::consts::PI {
                            angle_diff += 2.0 * std::f32::consts::PI;
                        }
                        if
                            angle_diff.abs() <= HALF_PLAYER_FOV &&
                            !seen_enemies.iter().any(|e: &SeenEnemy| e.enemy_handle == enemy_handle)
                        {
                            seen_enemies.push(SeenEnemy {
                                enemy_handle: enemy_handle,
                                relative_angle: angle_diff,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        RenderPlayerPOV::render_enemies(
            &z_buffer,
            self.player.pos,
            &seen_enemies,
            &self.enemies.positions,
            &self.enemies.animation_states
        );
        RenderPlayerPOV::render_weapon();
        gl_use_default_material();
        RenderMap::render_world_layout(&self.world_layout);
        RenderMap::render_player_and_enemies_on_map(self.player.pos, &self.enemies);
        RenderMap::render_rays(player_ray_origin, &raycast_result);
        draw_text(&format!("Raycasting FPS: {}", 1.0 / elapsed_time), 10.0, 30.0, 20.0, RED);
    }
}
#[macroquad::main(window_conf)]
async fn main() {
    let mut elapsed_time = 0.0;
    let mut world = World::default().await;
    loop {
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
