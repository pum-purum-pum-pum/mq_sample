use glam::{vec2, Vec2};
use miniquad::*;
use quad_gl::*;
use geo::Polygon;
use quad_rand as qrand;

use camera::Camera;
use drawing::{draw_window, DrawContext};
use gfx::{shadow::ShadowRenderer, triangle_sdf::TriangleSDF, deformed_texture::TextureRenderer};
use megaui::{hash, InputHandler};
use geometry::{brute_shadow_segment, generate_convex_polygon, shadow_shape};

mod camera;
mod gfx;
mod geometry;

const POLYS_N: usize = 4;

pub struct Stage {
    gl: QuadGl,
    polys: Vec<Polygon<f32>>,
    polys_pos: Vec<Vec2>, // no angle
    polys_vel: Vec<Vec2>,
    camera: Camera,
    mouse_pos: Vec2,
    shadow_renderer: ShadowRenderer,
    texture_renderer: TextureRenderer,
    triangle_sdf: TriangleSDF,
    drawing: DrawContext,
    speed_mult: f32,
    debug_drawing: bool,
}
fn main() {
    miniquad::start(conf::Conf::default(), |mut ctx| {
        miniquad::UserData::owning(Stage::new(&mut ctx), ctx)
    });
}

impl Stage {
    pub fn new(ctx: &mut Context) -> Self {
        let screen_size = ctx.screen_size();
        let mut polys = vec![];
        let mut polys_pos = vec![];
        let mut polys_vel = vec![];
        for i in 0..POLYS_N as i32 {
            let vel = vec2(qrand::gen_range(0.01, 0.02), qrand::gen_range(0.01, 0.02));
            polys_vel.push(vel);
            polys.push(generate_convex_polygon(10, 0.3));
            polys_pos.push(vec2(
                (2 * i - (POLYS_N + 1) as i32 / 2) as f32 * 2. / POLYS_N as f32,
                30. * vel.y(),
            ));
        }
        let shadow_renderer = ShadowRenderer::new(ctx);
        let texture_renderer = TextureRenderer::new(ctx);
        let triangle_sdf = TriangleSDF::new(ctx);

        Stage {
            gl: QuadGl::new(ctx),
            polys,
            polys_vel,
            polys_pos,
            camera: Camera::new(screen_size.0, screen_size.1),
            mouse_pos: vec2(0., 0.),
            shadow_renderer,
            texture_renderer,
            triangle_sdf,
            drawing: DrawContext::new(ctx),
            speed_mult: 1.,
            debug_drawing: false,
        }
    }

    /// Draw imgui and update parameters
    fn gui(&mut self) {
        // udpate params from gui
        let mut speed = self.speed_mult;
        let mut th = self.shadow_renderer.shadow_smooth_th;
        let mut light_size = self.shadow_renderer.light_size;
        let mut debug_drawing = self.debug_drawing;
        let mut sdf_edge = self.triangle_sdf.sdf_edge;
        let mut robo_transofrm_time = self.texture_renderer.time;
        draw_window(
            &mut self.drawing.ui,
            hash!(),
            vec2(0.5, 0.5),
            vec2(250., 250.),
            None,
            |ui| {
                ui.label(None, "Controls");
                if ui.button(None, "debug") {
                    debug_drawing = !debug_drawing;
                }
                ui.slider(hash!(), "Speed", 0f32..10f32, &mut speed);
                ui.slider(hash!(), "Shadow Border th", 0f32..1f32, &mut th);
                ui.slider(hash!(), "Light_size", 0f32..3f32, &mut light_size);
                ui.slider(hash!(), "SDF TRIANGLE", 0f32..1f32, &mut sdf_edge);
                ui.slider(hash!(), "Robo transform", 0f32..std::f32::consts::PI, &mut robo_transofrm_time);
            },
        );
        self.speed_mult = speed;
        self.shadow_renderer.shadow_smooth_th = th;
        self.shadow_renderer.light_size = light_size;
        self.debug_drawing = debug_drawing;
        self.triangle_sdf.sdf_edge = sdf_edge;
        self.texture_renderer.time = robo_transofrm_time;
    }
}

impl EventHandler for Stage {
    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) {
        self.camera.update_window(width, height)
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        _keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        _button: MouseButton,
        x: f32,
        y: f32,
    ) {
        self.drawing.ui.mouse_down((x, y));
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, _button: MouseButton, x: f32, y: f32) {
        self.drawing.ui.mouse_up((x, y));
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.mouse_pos = self.camera.unproject(vec2(x, y));
        self.shadow_renderer.light_pos = self.mouse_pos;
        self.drawing.ui.mouse_move((x, y));
    }

    fn mouse_wheel_event(&mut self, _ctx: &mut Context, x: f32, y: f32) {
        self.drawing.ui.mouse_wheel(x, y);
        self.camera.update_from_wheel(y);
    }

    fn update(&mut self, _ctx: &mut Context) {
        let y_shift = 2.;
        for (pos, vel) in self.polys_pos.iter_mut().zip(self.polys_vel.iter()) {
            *pos.y_mut() = (*pos.y_mut() + y_shift + self.speed_mult * vel.y()) % 5. - y_shift;
        }
    }

    fn draw(&mut self, ctx: &mut Context) {
        self.gui();
        ctx.begin_default_pass(PassAction::clear_color(0., 0., 0., 1.));
        self.gl.set_projection_matrix(self.camera.get_projection());
        self.shadow_renderer.clear_shadows_buffer();
        for (pos, poly) in self.polys_pos.iter().zip(self.polys.iter()) {
            draw_polygon(&mut self.gl, poly, *pos);
            let segment = brute_shadow_segment(&poly, *pos, self.mouse_pos);
            let shadow_points = shadow_shape(segment, self.mouse_pos, *pos);
            self.shadow_renderer.push_shadow(shadow_points);
            if self.debug_drawing {
                debug_drawing(&mut self.gl, self.mouse_pos, shadow_points);
            }
        }
        ctx.end_render_pass();

        let projection = self.camera.get_projection();
        self.texture_renderer.deform_texture(ctx);
        self.texture_renderer.draw(ctx, projection);
        self.triangle_sdf.draw(ctx, projection);


        let projection = self.camera.get_projection();
        self.shadow_renderer.reconstruct_buffers(ctx);
        self.shadow_renderer.offscreen_pass_draw(ctx, projection);
        self.shadow_renderer.draw(ctx);

        self.gl.draw(ctx);
        self.drawing.update_projection_matrix(ctx);
        self.drawing.perform_render_passes(ctx);
        ctx.commit_frame();
    }
}


/// draw shadows mesh
fn debug_drawing(gl: &mut QuadGl, mouse_pos: Vec2, shadow_points: [Vec2; 4]) {
    gl.draw_mode(DrawMode::Lines);
    let geom: Vec<_> = shadow_points
        .iter()
        .map(|p| Vertex::new(p.x(), p.y(), 0., 0., 0., BLUE))
        .collect();
    gl
        .geometry(&geom, &gen_line_indices(shadow_points.len() as u16));
    // draw triangle under mouse
    gl.draw_mode(DrawMode::Triangles);
    let pointer_size = 0.1;
    let (mx, my) = (mouse_pos.x(), mouse_pos.y());
    gl.geometry(
        &[
            Vertex::new(mx, my - pointer_size, 0., 0., 0., BLUE),
            Vertex::new(mx + pointer_size, my + pointer_size, 0., 0., 0., RED),
            Vertex::new(mx - pointer_size, my + pointer_size, 0., 0., 0., GREEN),
        ],
        &[0, 1, 2],
    );
}

/// Draw inner size of polygon
fn draw_polygon(gl: &mut QuadGl, poly: &Polygon<f32>, pos: Vec2) {
    gl.draw_mode(DrawMode::Triangles);
    let exterior: Vec<_> = poly
        .exterior()
        .points_iter()
        .map(|p| {
            Vertex::new(
                p.x() + pos.x(),
                p.y() + pos.y(),
                0.,
                0.,
                0.,
                Color([50, 50, 50, 255]),
            )
        })
        .collect();
    let indices = gen_triangulation_indices(exterior.len() as u16);
    gl.geometry(&exterior, &indices);
}

/// Exterior's indices of polygon vertices
/// [segment1.point1id, segment1.point2id,
///  segment2.point1id, segment2.point2id, ...]
fn gen_line_indices(length: u16) -> Vec<u16> {
    let mut indices = vec![];
    for i in 0..length {
        indices.push(i);
        indices.push((i + 1) % length)
    }
    indices
}

/// Exterior's indices of polygon vertices
/// [segment1.point1id, segment1.point2id, segment1.point3id
///  segment2.point1id, segment2.point2id, segment2.point3id ...]
fn gen_triangulation_indices(length: u16) -> Vec<u16> {
    let mut indices = vec![];
    for i in 1..length - 1 {
        indices.push(0);
        indices.push(i);
        indices.push((i + 1) % length)
    }
    indices
}
