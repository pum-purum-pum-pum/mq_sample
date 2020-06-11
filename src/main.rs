use geo::{Coordinate, LineString, Polygon};
use glam::{vec2, Vec2};
use miniquad::*;
use quad_gl::*;

use camera::Camera;
use gfx::shadow::ShadowRenderer;
use polygon::{generate_convex_polygon, shadow_segment, shadow_shape, shadow_segments, brute_shadow_segment};

mod camera;
mod gfx;
mod polygon;

const POLYS_N: usize = 4;

pub struct Stage {
    gl: QuadGl,
    polys: Vec<Polygon<f32>>,
    polys_pos: Vec<Vec2>, // no angle
    camera: Camera,
    mouse_pos: Vec2,
    shadow_renderer: ShadowRenderer,
}
fn main() {
    miniquad::start(conf::Conf::default(), |mut ctx| {
        miniquad::UserData::owning(Stage::new(&mut ctx), ctx)
    });
}

impl Stage {
    pub fn new(ctx: &mut Context) -> Self {
        let screen_size = ctx.screen_size();
        // let poly = generate_convex_polygon(10, 0.3);
        let mut polys = vec![];
        let mut polys_pos = vec![];
        for i in 0..POLYS_N as i32 {
            polys.push(generate_convex_polygon(10, 0.3));
            polys_pos.push(vec2((2 * i - (POLYS_N + 1) as i32 / 2) as f32 * 2. / POLYS_N as f32, 0.));
        }
        let shadow_renderer = ShadowRenderer::new(ctx);
        Stage {
            gl: QuadGl::new(ctx),
            polys,
            polys_pos,
            camera: Camera::new(screen_size.0, screen_size.1),
            mouse_pos: vec2(0., 0.),
            shadow_renderer,
        }
    }
}

impl EventHandler for Stage {
    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) {
        self.camera.update_window(width, height)
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: MouseButton, x: f32, y: f32) {
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32) {
        self.mouse_pos = self.camera.unproject(vec2(x, y));
    }

    fn mouse_wheel_event(&mut self, ctx: &mut Context, _x: f32, y: f32) {
        self.camera.update_from_wheel(y);
    }

    fn update(&mut self, ctx: &mut Context) {}

    fn draw(&mut self, ctx: &mut Context) {
        ctx.begin_default_pass(PassAction::clear_color(0., 0., 0., 1.));
        self.gl.set_projection_matrix(self.camera.get_projection());
        self.shadow_renderer.clear_shadows_buffer();
        for (pos, poly) in self.polys_pos.iter().zip(self.polys.iter()) {
            self.gl.draw_mode(DrawMode::Triangles);
            let exterior: Vec<_> = poly
                .exterior()
                .points_iter()
                .map(|p| Vertex::new(p.x() + pos.x(), p.y() + pos.y(), 0., 0., 0., WHITE))
                .collect();
            let indices = gen_triangulation_indices(exterior.len() as u16);
            self.gl.geometry(&exterior, &indices);
            
            self.gl.draw_mode(DrawMode::Lines);
            let segment = brute_shadow_segment(&poly, *pos, self.mouse_pos);

            let shadow_points = shadow_shape(segment, self.mouse_pos, *pos);
            self.shadow_renderer.push_shadow(shadow_points);
            // render debug shadow lines
            // let shadow: Vec<_> = shadow_shape(segment, self.mouse_pos, *pos)
            //     .iter()
            //     .map(|p| Vertex::new(p.x(), p.y(), 0., 0., 0., RED))
            //     .collect();
            // let shadow_indices = gen_line_indices(shadow.len() as u16);
            // self.gl.geometry(&shadow, &shadow_indices);

    
            self.gl.draw_mode(DrawMode::Triangles);
    
            let pointer_size = 0.1;
            let (mx, my) = (self.mouse_pos.x(), self.mouse_pos.y());
            self.gl.geometry(
                &[
                    Vertex::new(mx, my - pointer_size, 0., 0., 0., BLUE),
                    Vertex::new(mx + pointer_size, my + pointer_size, 0., 0., 0., RED),
                    Vertex::new(mx - pointer_size, my + pointer_size, 0., 0., 0., GREEN),
                ],
                &[0, 1, 2],
            );
        }
        // dbg!(self.shadow_renderer.shadows.len());
        self.shadow_renderer.reconstruct_buffers(ctx);
            
        self.shadow_renderer.draw(ctx, self.camera.get_projection());

        
        self.gl.draw(ctx);
        ctx.end_render_pass();
        ctx.commit_frame();
    }
}

fn gen_line_indices(length: u16) -> Vec<u16> {
    let mut indices = vec![];
    for i in 0..length {
        indices.push(i);
        indices.push((i + 1) % length)
    }
    indices
}

fn gen_triangulation_indices(length: u16) -> Vec<u16> {
    let mut indices = vec![];
    for i in 1..length - 1 {
        indices.push(0);
        indices.push(i);
        indices.push((i + 1) % length)
    }
    indices
}
