use std::collections::HashSet;

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
    render_data: RenderData, // will render weapon
}
#[derive(Clone, Copy)]
enum EntityType {
    None = 0,
    Wall = 1,
    Player = 2,
    Enemy = 3,
}
#[derive(Clone)]
struct RenderData {
    vertex_buffer: BufferId,
    indices: Vec<u16>,
    index_buffer: BufferId,
}

struct RenderDataCreator;
impl RenderDataCreator {
    fn render_data_for_rectangle(
        ctx: &mut Context,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: Vec4
    ) -> RenderData {
        let x = (x as f32) * (TILE_SIZE_X_PIXEL as f32);
        let y = (y as f32) * (TILE_SIZE_Y_PIXEL as f32);
        let width = width as f32;
        let height = height as f32;
        let half_width = width * 0.5 * (TILE_SIZE_X_PIXEL as f32);
        let half_height = height * 0.5 * (TILE_SIZE_Y_PIXEL as f32);
        let vertices = vec![
            Vec2::new(x - half_width, y - half_height),
            Vec2::new(x + half_width, y - half_height),
            Vec2::new(x + half_width, y + half_height),
            Vec2::new(x - half_width, y + half_height)
        ];
        let mut vertex_buffer_data: Vec<f32> = Vec::new();
        for vertex in vertices {
            vertex_buffer_data.push(vertex.x);
            vertex_buffer_data.push(vertex.y);
            vertex_buffer_data.extend_from_slice(&[color.x, color.y, color.z, color.w]);
        }

        let indices = vec![0, 1, 2, 0, 2, 3];
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertex_buffer_data)
        );
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices)
        );
        RenderData {
            vertex_buffer,
            indices,
            index_buffer,
        }
    }
}
struct RenderMapSystem;
impl RenderMapSystem {
    fn render(
        ctx: &mut dyn RenderingBackend,
        positions: &Vec<Vec2>,
        render_data: &Vec<RenderData>,
        pipelines: &Vec<Pipeline>
    ) {
        for (i, data) in render_data.iter().enumerate() {
            let bindings = Bindings {
                vertex_buffers: vec![data.vertex_buffer],
                index_buffer: data.index_buffer,
                images: vec![],
            };
            ctx.apply_pipeline(&pipelines[0]);
            ctx.apply_bindings(&bindings);
            let screen_size = Vec2::new(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
            let model = Mat4::from_translation(Vec3::new(positions[i].x, -positions[i].y, 0.0));
            ctx.apply_uniforms(UniformsSource::table(&(model, screen_size)));
            ctx.draw(0, data.indices.len() as i32, 1);
        }
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
struct RaycastResult {
    hit: bool,
    distance: f32,
    entity: EntityType,
}
struct RaycastSystem;
impl RaycastSystem {
    fn update(origin: Vec2, angle: f32, map: &Vec<Vec<EntityType>>, rays: usize, max_distance: usize) -> Vec<RaycastResult> {
        let mut results: Vec<RaycastResult> = Vec::new();
        let mut pos = origin;
        let mut angle = angle;
        let steps = angle / rays as f32;
        for i in 0..rays {
            let current_angle = angle + (steps * i as f32);
            let mut distance = 0.0;
            let mut hit = false;
            let mut entity = EntityType::None;
            todo!();
        }
        return results;
    }
    fn visualize0() -> RenderData {
        todo!()
    }
}
struct Enemies {
    positions: Vec<Vec2>,
    velocities: Vec<Vec2>,
    health: Vec<f32>,
    render_data: Vec<RenderData>,
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
            }
            else if x == y {
                layout[y][x] = EntityType::Wall;
            }
        }
    }
    layout
}
struct Walls {
    positions: Vec<Vec2>,
    render_datas: Vec<RenderData>,
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
            render_datas: Vec::new(),
        };

        for y in 0..WORLD_HEIGHT as usize {
            for x in 0..WORLD_WIDTH as usize {
                match layout[y][x] {
                    EntityType::Enemy => {
                        enemies.positions.push(Vec2::new(x as f32, y as f32));
                        enemies.velocities.push(Vec2::new(0.0, 0.0));
                        enemies.health.push(100.0);
                        enemies.render_data.push(
                            RenderDataCreator::render_data_for_rectangle(
                                &mut *ctx,
                                x,
                                y,
                                1,
                                1,
                                Vec4::new(0.0, 1.0, 0.0, 1.0)
                            )
                        );
                    }
                    EntityType::Wall => {
                        walls.positions.push(Vec2::new(x as f32, y as f32));
                        walls.render_datas.push(
                            RenderDataCreator::render_data_for_rectangle(
                                &mut *ctx,
                                x,
                                y,
                                1,
                                1,
                                Vec4::new(1.0, 0.0, 0.0, 1.0)
                            )
                        );
                    }
                    _ => {}
                }
            }
        }
        println!("Enemies: {}", enemies.positions.len());
        println!("Walls: {}", walls.positions.len());
        Self {
            player: Player {
                pos: Vec2::new((WORLD_WIDTH as f32) / 2.0, (WORLD_HEIGHT as f32) / 2.0),
                vel: Vec2::new(0.0, 0.0),
                angle: 0.0,
                health: 100.0,
                render_data: RenderDataCreator::render_data_for_rectangle(
                    &mut *ctx,
                    (WORLD_WIDTH as usize) / 2,
                    (WORLD_HEIGHT as usize) / 2,
                    1,
                    1,
                    Vec4::new(0.0, 0.0, 1.0, 1.0)
                ),
            },
            map: layout,
            enemies,
            walls,
        }
    }
    fn update(&mut self) {
        MovementSystem::update(&mut self.enemies.positions, &mut self.enemies.velocities);
        self.player.pos += self.player.vel * PHYSICS_FRAME_TIME;
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
                            UniformDesc::new("screen_size", UniformType::Float2),
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
        let world = World::default(&mut *ctx);
        Self {
            world,
            ctx,
            physics_elapsed_time: 0.0,
            physics_last_time: date::now(),
            draw_last_time: date::now(),
            pressed_keys: HashSet::new(),
            pipelines: vec![default_pipeline],
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
            // let start = date::now();
            self.world.update();
            // let end = date::now();
            // println!("Physics update took: {}", end - start);
            // println!("FPS: {}", 1.0 / (end - start));
            // println!("Asteroids: {}", self.world.asteroids.positions.len());
            self.physics_elapsed_time -= PHYSICS_FRAME_TIME;
        }
    }
    fn draw(&mut self) {
        self.ctx.clear(Some((0.0, 0.0, 0.0, 1.0)), None, None);
        let dt = (date::now() - self.draw_last_time) as f32;
        self.draw_last_time = date::now();
        match self.ctx.info().backend {
            Backend::OpenGl => {
                // self.ctx.begin_default_pass(Default::default());
                RenderMapSystem::render(
                    &mut *self.ctx,
                    &self.world.enemies.positions,
                    &self.world.enemies.render_data,
                    &self.pipelines
                );

                RenderMapSystem::render(
                    &mut *self.ctx,
                    &vec![self.world.player.pos],
                    &vec![self.world.player.render_data.clone()],
                    &self.pipelines
                );

                RenderMapSystem::render(
                    &mut *self.ctx,
                    &self.world.walls.positions,
                    &self.world.walls.render_datas,
                    &self.pipelines
                );

                // self.ctx.end_render_pass();
                // self.ctx.commit_frame();
            }
            _ => {}
        }
        self.ctx.commit_frame();
    }
    fn key_down_event(&mut self, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) {
        self.pressed_keys.insert(keycode);
        self.world.player.vel = Self::calculate_velocity(&self.pressed_keys) * 100.0;
        self.world.player.angle = self.world.player.vel.y.atan2(self.world.player.vel.x);
    }

    fn key_up_event(&mut self, keycode: KeyCode, _keymods: KeyMods) {
        self.pressed_keys.remove(&keycode);
        self.world.player.vel = Self::calculate_velocity(&self.pressed_keys) * 100.0;
        self.world.player.angle = self.world.player.vel.y.atan2(self.world.player.vel.x);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {}
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
