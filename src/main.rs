use std::{
    char::REPLACEMENT_CHARACTER,
    collections::HashSet,
    f32::consts::PI,
    marker::PhantomData, thread::sleep, time::Duration,
};

use config::config::{
    PHYSICS_FRAME_TIME,
    SCREEN_HEIGHT,
    SCREEN_WIDTH,
    TILE_SIZE_X_PIXEL,
    TILE_SIZE_Y_PIXEL,
    WORLD_HEIGHT,
    WORLD_WIDTH,
};
use glam::{ Mat4, Vec2, Vec3, Vec4 };
use miniquad::{
    conf::Conf,
    date,
    window,
    Backend,
    Bindings,
    BufferId,
    BufferLayout,
    BufferSource,
    BufferType,
    BufferUsage,
    Context,
    EventHandler,
    KeyCode,
    KeyMods,
    MouseButton,
    Pipeline,
    PipelineParams,
    RenderingBackend,
    ShaderMeta,
    ShaderSource,
    UniformBlockLayout,
    UniformDesc,
    UniformType,
    UniformsSource,
    VertexAttribute,
    VertexFormat,
};

pub mod config;

#[derive(Clone)]
struct Player {
    pos: Vec2,
    vel: Vec2,
    angle: f32,
    health: f32,
    // render_data: VertexRenderData, // will render weapon
}
#[derive(Clone, Copy, Debug)]
enum EntityType {
    None = 0,
    Wall = 1,
    Player = 2,
    Enemy = 3,
}
#[derive(Clone)]
struct VertexRenderData {
    vertex_buffer: BufferId,
    indices: Vec<u16>,
    index_buffer: BufferId,
}
#[derive(Clone)]
struct PointRenderData {
    points_buffer: BufferId,
    indices: Vec<u16>,
    index_buffer: BufferId,
}
struct RectangleRenderSystem {
    vertex_buffer: BufferId,
    index_buffer: BufferId,
}

impl RectangleRenderSystem {
    fn new(ctx: &mut dyn RenderingBackend) -> Self {
        let vertices = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0)
        ];

        let mut vertex_buffer_data: Vec<f32> = Vec::new();
        for vertex in vertices {
            vertex_buffer_data.push(vertex.x);
            vertex_buffer_data.push(vertex.y);
        }

        let indices = vec![0, 1, 2, 0, 2, 3];
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Dynamic,
            BufferSource::slice(&vertex_buffer_data)
        );
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices)
        );

        RectangleRenderSystem {
            vertex_buffer,
            index_buffer,
        }
    }

    fn render_rectangle(
        &self,
        ctx: &mut dyn RenderingBackend,
        position1: Vec2,
        position2: Vec2,
        color: Vec4
    ) {
        let screen_position1 = Vec2::new(
            position1.x * (TILE_SIZE_X_PIXEL as f32),
            position1.y * (TILE_SIZE_Y_PIXEL as f32)
        );
        let screen_position2 = Vec2::new(
            position2.x * (TILE_SIZE_X_PIXEL as f32),
            position2.y * (TILE_SIZE_Y_PIXEL as f32)
        );

        let width = (screen_position2.x - screen_position1.x).abs();
        let height = (screen_position2.y - screen_position1.y).abs();

        let translation = Mat4::from_translation(
            Vec3::new(screen_position1.x, screen_position1.y, 0.0)
        );
        let scale = Mat4::from_scale(Vec3::new(width, height, 1.0));
        let model = translation * scale;

        let bindings = Bindings {
            vertex_buffers: vec![self.vertex_buffer.clone()],
            index_buffer: self.index_buffer.clone(),
            images: vec![],
        };
        ctx.apply_bindings(&bindings);
        let screen_size = Vec2::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
        ctx.apply_uniforms(UniformsSource::table(&(model, screen_size, color)));
        ctx.draw(0, 6, 1);
    }
}
struct MovementSystem;
impl MovementSystem {
    fn update(positions: &mut Vec<Vec2>, velocities: &mut Vec<Vec2>) {
        for (pos, vel) in positions.iter_mut().zip(velocities.iter_mut()) {
            *pos += *vel * PHYSICS_FRAME_TIME;
        }
    }
}
struct KeepInBoundsSystem;
impl KeepInBoundsSystem {
    fn update(positions: &mut Vec<Vec2>) {
        for pos in positions.iter_mut() {
            if pos.x < 0.0 {
                pos.x = 0.0;
            }
            if pos.x > ((WORLD_WIDTH - 1) as f32) {
                pos.x = (WORLD_WIDTH - 1) as f32;
            }
            if pos.y < 0.0 {
                pos.y = 0.0;
            }
            if pos.y > ((WORLD_HEIGHT - 1) as f32) {
                pos.y = (WORLD_HEIGHT - 1) as f32;
            }
        }
    }
    fn update_player(position: &mut Vec2) {
        if position.x < 0.0 {
            position.x = 0.0;
        }
        if position.x > ((WORLD_WIDTH - 1) as f32) {
            position.x = (WORLD_WIDTH - 1) as f32;
        }
        if position.y < 0.0 {
            position.y = 0.0;
        }
        if position.y > ((WORLD_HEIGHT - 1) as f32) {
            position.y = (WORLD_HEIGHT - 1) as f32;
        }
    }
}
struct WallCollisionSystem;
impl WallCollisionSystem {
    fn update(positions: &mut Vec<Vec2>, walls: &Vec<Vec2>) {
        for pos in positions.iter_mut() {
            for wall in walls.iter() {
                let point_1 = Vec2::new(wall.x + 0.5, wall.y + 0.5);
                let point_2 = Vec2::new(pos.x - 0.5, pos.y - 0.5);
                let distance = point_1.distance(point_2);
                if distance < 0.5 {
                    let normal = (point_2 - point_1).normalize();
                    *pos += normal * (0.5 - distance);
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

            if distance_x <= 1.0 && distance_y <= 1.0 {
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
    fn raycast_and_visualize_tiles_traversed(
        ctx: &mut dyn RenderingBackend,
        renderer: &RectangleRenderSystem,
        origin: Vec2,
        angle: f32,
        rays: usize,
        max_tiles: usize,
        map: &[[EntityType; WORLD_WIDTH as usize]; WORLD_HEIGHT as usize]
    ) -> Vec<RaycastResult> {
        let mut res: Vec<RaycastResult> = Vec::new();
        const MAX_ANGLE: f32 = PI / 6.0;
        let angle_step_size = MAX_ANGLE / (rays as f32);
        let half_rays = rays / 2;
        const EPSILON: f32 = 1e-6;

        for ray_idx in 0..rays {
            let curr_angle = angle - (half_rays as f32) * angle_step_size + (ray_idx as f32) * angle_step_size;
            let mut curr_pos = origin;
            let direction = Vec2::new(curr_angle.cos() + EPSILON, curr_angle.sin() + EPSILON);
            
            let mut map_pos = curr_pos.trunc();
            let delta_dist = Vec2::new( // how far ray has to move to go from one x/y side to the next x/y side
                (1.0 / direction.x).abs(),
                (1.0 / direction.y).abs()
            );
            let step = Vec2::new( // tile based step size
                if direction.x < 0.0 { -1.0 } else { 1.0 },
                if direction.y < 0.0 { -1.0 } else { 1.0 }
            );
            let mut side_dist = Vec2::new( // next x or y side distance, stays constant until we hit that side 
                (if direction.x < 0.0 { curr_pos.x - map_pos.x } else { map_pos.x + 1.0 - curr_pos.x }) * delta_dist.x,
                (if direction.y < 0.0 { curr_pos.y - map_pos.y } else { map_pos.y + 1.0 - curr_pos.y }) * delta_dist.y
            );

            for _ in 0..max_tiles {
                if map_pos.x < 0.0 || map_pos.x >= (WORLD_WIDTH as f32) || 
                   map_pos.y < 0.0 || map_pos.y >= (WORLD_HEIGHT as f32) {
                    break;
                }

                renderer.render_rectangle(
                    ctx,
                    map_pos,
                    map_pos + Vec2::new(1.0, 1.0),
                    Vec4::new(1.0, 1.0, 0.0, 1.0)
                );

                let is_x_side = side_dist.x < side_dist.y;
                if is_x_side {
                    side_dist.x += delta_dist.x;
                    map_pos.x += step.x;
                } else {
                    side_dist.y += delta_dist.y;
                    map_pos.y += step.y;
                }

                let map_idx_x = map_pos.x as usize;
                let map_idx_y = map_pos.y as usize;

                match map.get(map_idx_y).and_then(|row| row.get(map_idx_x)) {
                    Some(EntityType::Wall) => {
                        res.push(RaycastResult {
                            distance: curr_pos.distance(origin),
                            entity_pos: map_pos,
                            entity: EntityType::Wall,
                        });
                        break;
                    }
                    Some(EntityType::Enemy) => {
                        res.push(RaycastResult {
                            distance: curr_pos.distance(origin),
                            entity_pos: map_pos,
                            entity: EntityType::Enemy,
                        });
                        break;
                    }
                    _ => {}
                }

                // Update to next tile
                curr_pos = if is_x_side {
                    Vec2::new(map_pos.x + (if step.x > 0.0 { 0.0 } else { 1.0 }), curr_pos.y + direction.y * side_dist.x)
                } else {
                    Vec2::new(curr_pos.x + direction.x * side_dist.y, map_pos.y + (if step.y > 0.0 { 0.0 } else { 1.0 }))
                };
            }
        }
        res
    }
}
struct RaycastResult {
    distance: f32,
    entity_pos: Vec2,
    entity: EntityType,
}

struct Enemies {
    positions: Vec<Vec2>,
    velocities: Vec<Vec2>,
    health: Vec<f32>,
    render_data: Vec<VertexRenderData>,
}
fn create_world_layout() -> [[EntityType; WORLD_WIDTH as usize]; WORLD_HEIGHT as usize] {
    let mut layout: [[EntityType; WORLD_WIDTH as usize]; WORLD_HEIGHT as usize] = [
        [EntityType::None; WORLD_WIDTH as usize];
        WORLD_HEIGHT as usize
    ];
    for y in 0..WORLD_HEIGHT as usize {
        for x in 0..WORLD_WIDTH as usize {
            if
                x == 0 ||
                x == (WORLD_WIDTH as usize) - 1 ||
                y == 0 ||
                y == (WORLD_HEIGHT as usize) - 1
            {
                layout[y][x] = EntityType::Wall;
            } else if x == y {
                layout[y][x] = EntityType::Wall;
            }
        }
    }
    layout
}
struct Walls {
    positions: Vec<Vec2>,
}
struct World {
    player: Player,
    map: [[EntityType; WORLD_WIDTH as usize]; WORLD_HEIGHT as usize],
    enemies: Enemies,
    walls: Walls,
}
impl World {
    fn default(ctx: &mut Context) -> Self {
        let layout = create_world_layout();
        let mut enemies = Enemies {
            positions: Vec::new(),
            velocities: Vec::new(),
            health: Vec::new(),
            render_data: Vec::new(),
        };
        let mut walls = Walls {
            positions: Vec::new(),
        };

        for y in 0..layout.len() {
            for x in 0..layout[y].len() {
                match layout[y][x] {
                    EntityType::Enemy => {
                        enemies.positions.push(Vec2::new(x as f32, y as f32));
                        enemies.velocities.push(Vec2::new(0.0, 0.0));
                        enemies.health.push(100.0);
                    }
                    EntityType::Wall => {
                        walls.positions.push(Vec2::new(x as f32, y as f32));
                    }
                    _ => {}
                }
            }
        }
        Self {
            player: Player {
                pos: Vec2::new(2.0, 1.0),
                vel: Vec2::new(0.0, 0.0),
                angle: 0.0,
                health: 100.0,
            },
            map: layout,
            enemies,
            walls,
        }
    }
    fn update(&mut self) {
        MovementSystem::update(&mut self.enemies.positions, &mut self.enemies.velocities);
        self.player.pos += self.player.vel * PHYSICS_FRAME_TIME;
        KeepInBoundsSystem::update_player(&mut self.player.pos);
        WallCollisionSystem::update_player(&mut self.player.pos, &self.walls.positions);
    }
}

struct Stage {
    world: World,
    ctx: Box<dyn RenderingBackend>,
    physics_elapsed_time: f32,
    physics_last_time: f64,
    draw_last_time: f64,
    pressed_keys: HashSet<KeyCode>,
    pipelines: Vec<Pipeline>,
    render_system_rect: RectangleRenderSystem,
}

impl Stage {
    fn new() -> Self {
        let mut ctx = window::new_rendering_backend();

        let default_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: include_str!("./shaders/vertex.glsl"),
                    fragment: include_str!("./shaders/fragment.glsl"),
                },
                ShaderMeta {
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("model", UniformType::Mat4),
                            UniformDesc::new("screen_size", UniformType::Float2)
                        ],
                    },
                    images: vec![],
                }
            )
            .expect("Failed to create shader");
        let default_pipeline = ctx.new_pipeline(
            &[
                BufferLayout {
                    stride: 24,
                    ..Default::default()
                },
            ],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("color0", VertexFormat::Float4),
            ],
            default_shader,
            PipelineParams::default()
        );
        let line_pipeline = ctx.new_pipeline(
            &[
                BufferLayout {
                    stride: 24,
                    ..Default::default()
                },
            ],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("color0", VertexFormat::Float4),
            ],
            default_shader,
            PipelineParams {
                primitive_type: miniquad::PrimitiveType::Lines,
                ..Default::default()
            }
        );
        let rectangle_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: include_str!("./shaders/rectangle_vertex.glsl"),
                    fragment: include_str!("./shaders/fragment.glsl"),
                },
                ShaderMeta {
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("model", UniformType::Mat4),
                            UniformDesc::new("screen_size", UniformType::Float2),
                            UniformDesc::new("rect_color", UniformType::Float4)
                        ],
                    },
                    images: vec![],
                }
            )
            .expect("Failed to create shader");
        let rectangle_pipeline = ctx.new_pipeline(
            &[
                BufferLayout {
                    stride: 8,
                    ..Default::default()
                },
            ],
            &[VertexAttribute::new("pos", VertexFormat::Float2)],
            rectangle_shader,
            PipelineParams {
                primitive_type: miniquad::PrimitiveType::Triangles,
                ..Default::default()
            }
        );
        let world = World::default(&mut *ctx);
        let render_system_rect = RectangleRenderSystem::new(&mut *ctx);
        Self {
            world,
            ctx,
            physics_elapsed_time: 0.0,
            physics_last_time: date::now(),
            draw_last_time: date::now(),
            pressed_keys: HashSet::new(),
            pipelines: vec![default_pipeline, line_pipeline, rectangle_pipeline],
            render_system_rect,
        }
    }
    fn calculate_velocity(pressed_keys: &HashSet<KeyCode>) -> Vec2 {
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        if pressed_keys.contains(&KeyCode::W) {
            y -= 1.0;
        }
        if pressed_keys.contains(&KeyCode::S) {
            y += 1.0;
        }
        if pressed_keys.contains(&KeyCode::A) {
            x -= 1.0;
        }
        if pressed_keys.contains(&KeyCode::D) {
            x += 1.0;
        }
        if x == 0.0 && y == 0.0 {
            return Vec2::new(0.0, 0.0);
        }
        Vec2::new(x, y).normalize()
    }
}
impl EventHandler for Stage {
    fn update(&mut self) {
        self.physics_elapsed_time += (date::now() - self.physics_last_time) as f32;
        self.physics_last_time = date::now();

        while self.physics_elapsed_time >= PHYSICS_FRAME_TIME {
            self.world.update();
            self.physics_elapsed_time -= PHYSICS_FRAME_TIME;
        }
    }
    fn draw(&mut self) {
        self.ctx.clear(Some((0.0, 0.0, 0.0, 1.0)), None, None);
        let dt = (date::now() - self.draw_last_time) as f32;
        self.draw_last_time = date::now();

        match self.ctx.info().backend {
            Backend::OpenGl => {
                self.ctx.apply_pipeline(&self.pipelines[2]);
                for wall in &self.world.walls.positions {
                    self.render_system_rect.render_rectangle(
                        &mut *self.ctx,
                        *wall,
                        *wall + Vec2::new(1.0, 1.0),
                        Vec4::new(1.0, 0.0, 0.0, 1.0)
                    );
                }
                self.ctx.apply_pipeline(&self.pipelines[2]);

                self.render_system_rect.render_rectangle(
                    &mut *self.ctx,
                    self.world.player.pos,
                    self.world.player.pos + Vec2::new(1.0, 1.0),
                    Vec4::new(0.0, 0.0, 1.0, 1.0)
                );
                self.ctx.apply_pipeline(&self.pipelines[2]);

                let res = RaycastSystem::raycast_and_visualize_tiles_traversed(
                    &mut *self.ctx,
                    &self.render_system_rect,
                    self.world.player.pos + Vec2::new(0.5, 0.5),
                    self.world.player.angle,
                    3,
                    5,
                    &self.world.map
                );
                if res.len() > 0 {
                    for ray in res {
                        println!("Ray hit entity: {:?} at pos: {:?} with distance: {}", ray.entity, ray.entity_pos.trunc(), ray.distance);
                        self.render_system_rect.render_rectangle(
                            &mut *self.ctx,
                            ray.entity_pos,
                            ray.entity_pos + Vec2::new(1.0, 1.0),
                            Vec4::new(0.0, 1.0, 0.0, 1.0)
                        );
                    }
                }
            }
            _ => {}
        }
        self.ctx.commit_frame();
    }
    fn key_down_event(&mut self, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) {
        self.pressed_keys.insert(keycode);
        self.world.player.vel = Self::calculate_velocity(&self.pressed_keys);
    }

    fn key_up_event(&mut self, keycode: KeyCode, _keymods: KeyMods) {
        self.pressed_keys.remove(&keycode);
        self.world.player.vel = Self::calculate_velocity(&self.pressed_keys);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.world.player.angle = 0.33 * 2.0 * PI;
    }
    fn mouse_button_down_event(&mut self, _button: MouseButton, _x: f32, _y: f32) {}
}
fn main() {
    miniquad::start(
        Conf {
            window_title: "DoomR".to_owned(),
            window_width: SCREEN_WIDTH as i32,
            window_height: SCREEN_HEIGHT as i32,
            ..Default::default()
        },
        || Box::new(Stage::new())
    );
}
