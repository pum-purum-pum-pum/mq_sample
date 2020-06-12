use glam::{vec2, vec3, vec4, Mat4, Vec2};
pub const MAX_ZOOM: f32 = 0.8;
pub const MIN_ZOOM: f32 = 0.05;
pub const INIT_ZOOM: f32 = 0.2;

/// Simple orthographic camera
pub struct Camera {
    pub position2d: Vec2,
    pub zoom: f32,
    window_width: f32,
    window_height: f32,
}

#[allow(dead_code)]
impl Camera {
    pub fn new(window_width: f32, window_height: f32) -> Self {
        Camera {
            position2d: vec2(0., 0.),
            zoom: INIT_ZOOM,
            window_width,
            window_height,
        }
    }

    pub fn update_window(&mut self, width: f32, height: f32) {
        self.window_width = width;
        self.window_height = height;
    }

    pub fn get_projection(&self) -> Mat4 {
        let w = 1. / self.zoom;
        let h = (self.window_height / self.window_width) / self.zoom;
        let proj = Mat4::orthographic_rh_gl(
            -w / 2., // left
            w / 2.,  // right
            -h / 2., // bottom
            h / 2.,  // top
            1.,      // near
            0.,      // far
        );
        let eye = vec3(self.position2d.x(), self.position2d.y(), 1.);
        let center = vec3(self.position2d.x(), self.position2d.y(), 0.0);
        let up = vec3(0.0, 1.0, 0.0);
        let view = Mat4::look_at_rh(eye, center, up);
        proj * view
    }

    /// Project into [0, 1] x [0, 1] space. NOTE It creates projection matrix inside
    pub fn project(&self, point: Vec2) -> Vec2 {
        let (width, height) = (self.window_width, self.window_height);
        let mvp = self.get_projection();
        let projected = mvp * vec4(point.x(), point.y(), 0., 1.);
        vec2(
            (projected.x() + 1.) * width / 2.,
            (1. - projected.y()) * height / 2.,
        )
    }

    /// Get world coordinates from (x, y) \in [0, 1] x [0, 1] screen coordinates. 
    pub fn unproject(&self, point: Vec2) -> Vec2 {
        let (x, y) = (point.x(), point.y());
        let (width, height) = (self.window_width, self.window_height);
        // coords are in cube with corners [-1, -1, -1], [1, 1, 1] after orthographic projection
        let sx = -1. + 2. * x / width;
        let sy = 1. - 2. * y / height;
        // apply inverse matrix to point on a surface
        let unproject_pos = self.get_projection().inverse() * vec4(sx, sy, 0., 1.);
        vec2(unproject_pos.x(), unproject_pos.y())
    }

    /// Udpate zoom from wheel y diff
    pub fn update_from_wheel(&mut self, value: f32) {
        self.zoom *= f32::powf(1.2, value);
        self.zoom = self.zoom.min(MAX_ZOOM).max(MIN_ZOOM);
    }
}
