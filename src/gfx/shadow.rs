use crate::geometry::projective_textures;
use glam::{vec2, Mat4, Vec2, Vec3};
use miniquad::*;

/// Max number of shadow vertices (#shadows = MAX_SHADOW_VERTICES / 4)
const MAX_SHADOW_VERTICES_BYTES: usize = 1000;
/// Max number of shadow indices (#shadows = MAX_SHADOW_INDICES / 6)
const MAX_SHADOW_INDICES_BYTES: usize = 1500;

const TEXTURE_SIZE: u32 = 1024;

#[repr(C)]
struct ShadowVertex {
    pos: Vec2,
    normal: Vec3,
}

#[repr(C)]
struct GeoVertex {
    pos: Vec2,
}

#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

pub struct ShadowRenderer {
    // offscreen pipeline -- render shadow map into texture
    offscreen_pipeline: Pipeline,
    offscreen_bindings: Bindings,
    offscreen_light_pipeline: Pipeline,
    offscreen_light_bindings: Bindings,
    offscreen_pass: RenderPass,
    // display pipeline -- process shadow map and draw scene
    display_pipeline: Pipeline,
    display_bindings: Bindings,
    shadows: Vec<[Vec2; 4]>,
    vertices: Vec<ShadowVertex>,
    indices: Vec<u16>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    pub shadow_smooth_th: f32,
    pub light_pos: Vec2,
    pub light_size: f32,
}

impl ShadowRenderer {
    /// Use this method to add shadow geometry(4 vertices)
    pub fn push_shadow(&mut self, shadow: [Vec2; 4]) {
        self.shadows.push(shadow);
    }

    /// Remove all shadows pushed in by push_shadow
    pub fn clear_shadows_buffer(&mut self) {
        self.shadows.clear()
    }

    /// Update rendering pipeline with new shadows
    pub fn reconstruct_buffers(&mut self, ctx: &mut Context) {
        self.vertices.clear();
        self.indices.clear();
        self.vertices.shrink_to_fit();
        self.indices.shrink_to_fit();
        let normals = [vec2(1., 0.), vec2(1., 1.), vec2(0., 1.), vec2(0., 0.)];
        for shadow in self.shadows.iter() {
            let newi = vec![0, 1, 2, 3, 2, 0];
            // new indices start from "shift" position
            let shift = self.vertices.len();
            for idx in newi.iter() {
                self.indices.push((shift + *idx) as u16);
            }

            let projective_normals = projective_textures(shadow, &normals);
            for (v, n) in shadow.iter().zip(projective_normals.iter()) {
                self.vertices.push(ShadowVertex {
                    pos: *v,
                    normal: *n,
                });
            }
        }
        self.vertex_buffer.update(ctx, &self.vertices);
        self.index_buffer.update(ctx, &self.indices);
    }

    pub fn new(ctx: &mut Context) -> ShadowRenderer {
        let color_img = Texture::new_render_texture(
            ctx,
            TextureParams {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                format: TextureFormat::RGBA8,
                ..Default::default()
            },
        );
        let depth_img = Texture::new_render_texture(
            ctx,
            TextureParams {
                width: TEXTURE_SIZE,
                height: TEXTURE_SIZE,
                format: TextureFormat::Depth,
                ..Default::default()
            },
        );
        // offscreen pipeline
        // light (simple point light)
        let size = 1000.;
        #[rustfmt::skip]
        let vertices: [GeoVertex; 4] = [
            GeoVertex { pos : vec2(-size, -size)},
            GeoVertex { pos : vec2(size, -size )},
            GeoVertex { pos : vec2(  size,  size)},
            GeoVertex { pos : vec2( -size,  size)},
        ];
        let indices: &[u16] = &[0, 1, 2, 3, 2, 0];

        let vertex_light_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);
        let index_light_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);

        let offscreen_light_bindings = Bindings {
            vertex_buffers: vec![vertex_light_buffer],
            index_buffer: index_light_buffer,
            images: vec![],
        };

        let offscreen_light_shader = Shader::new(
            ctx,
            offscreen_light_shader::VERTEX,
            offscreen_light_shader::FRAGMENT,
            offscreen_light_shader::META,
        )
        .unwrap();
        let offscreen_light_pipeline = Pipeline::new(
            ctx,
            &[BufferLayout::default()],
            &[VertexAttribute::new("pos", VertexFormat::Float2)],
            offscreen_light_shader,
        );
        // shadows
        // Note that glitch when changing shadow segment
        // Projective textures trick is not used for light (at least that way) in real life, but its good enough for this demo.
        let (offscreen_pipeline, offscreen_bindings, vertex_buffer, index_buffer, offscreen_pass) = {
            let offscreen_pass = RenderPass::new(ctx, color_img, depth_img);
            let vertex_buffer =
                Buffer::stream(ctx, BufferType::VertexBuffer, MAX_SHADOW_VERTICES_BYTES);
            let index_buffer =
                Buffer::stream(ctx, BufferType::IndexBuffer, MAX_SHADOW_INDICES_BYTES);

            let offscreen_bindings = Bindings {
                vertex_buffers: vec![vertex_buffer],
                index_buffer,
                images: vec![],
            };

            let offscreen_shader = Shader::new(
                ctx,
                offscreen_shader::VERTEX,
                offscreen_shader::FRAGMENT,
                offscreen_shader::META,
            )
            .unwrap();
            let offscreen_pipeline = Pipeline::new(
                ctx,
                &[BufferLayout::default()],
                &[
                    VertexAttribute::new("pos", VertexFormat::Float2),
                    VertexAttribute::new("normal", VertexFormat::Float3),
                ],
                offscreen_shader,
            );
            (
                offscreen_pipeline,
                offscreen_bindings,
                vertex_buffer,
                index_buffer,
                offscreen_pass,
            )
        };

        // display pipeline
        let size = 1.;
        // Screen size rectangle (without projection)
        #[rustfmt::skip]
        let vertices: [Vertex; 4] = [
            Vertex { pos : vec2(-size, -size), uv: vec2( 0., 0.) },
            Vertex { pos : vec2(size, -size ), uv: vec2(1., 0.) },
            Vertex { pos : vec2(  size,  size), uv: vec2( 1., 1.) },
            Vertex { pos : vec2( -size,  size), uv: vec2( 0., 1.) },
        ];
        let display_vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        let indices: &[u16] = &[0, 1, 2, 3, 2, 0];
        let display_index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);
        let display_bindings = Bindings {
            vertex_buffers: vec![display_vertex_buffer],
            index_buffer: display_index_buffer,
            images: vec![color_img],
        };
        let default_shader = Shader::new(
            ctx,
            display_shader::VERTEX,
            display_shader::FRAGMENT,
            display_shader::META,
        )
        .unwrap();
        let display_pipeline = Pipeline::with_params(
            ctx,
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("uv0", VertexFormat::Float2),
            ],
            default_shader,
            PipelineParams {
                depth_write: false,
                ..Default::default()
            },
        );

        ShadowRenderer {
            offscreen_pipeline,
            display_pipeline,
            offscreen_light_pipeline,
            offscreen_light_bindings,
            offscreen_pass,
            display_bindings,
            offscreen_bindings,
            vertex_buffer,
            index_buffer,
            shadows: vec![],
            vertices: vec![],
            indices: vec![],
            shadow_smooth_th: 0.1,
            light_pos: vec2(0., 0.),
            light_size: 1f32,
        }
    }

    pub fn offscreen_pass_draw(&mut self, ctx: &mut Context, projection: Mat4) {
        ctx.begin_pass(self.offscreen_pass, PassAction::default());
        // shadows
        ctx.apply_pipeline(&self.offscreen_pipeline);
        ctx.apply_bindings(&self.offscreen_bindings);
        ctx.apply_uniforms(&offscreen_shader::Uniforms {
            projection,
            th: self.shadow_smooth_th,
        });
        ctx.draw(0, self.indices.len() as i32, 1);
        // light
        ctx.apply_pipeline(&self.offscreen_light_pipeline);
        ctx.apply_bindings(&self.offscreen_light_bindings);
        ctx.apply_uniforms(&offscreen_light_shader::Uniforms {
            projection,
            light: self.light_pos,
            size: self.light_size,
        });
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();
    }

    pub fn draw(&mut self, ctx: &mut Context) {
        ctx.begin_default_pass(PassAction::Nothing);
        ctx.apply_pipeline(&self.display_pipeline);
        ctx.apply_bindings(&self.display_bindings);
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();
    }
}

/// Vertex and Fragment shader to render light (used in offscreen pipeline)
mod offscreen_light_shader {
    use glam::Vec2;
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    varying highp vec2 fpos;
    
    uniform mat4 projection;

    void main() {
        gl_Position = projection * vec4(pos, 0, 1);
        fpos = pos;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    uniform lowp vec2 light;
    uniform lowp float size;

    varying highp vec2 fpos;

    void main() {
        lowp vec2 dir = fpos - light;
        lowp float dst = size / (dir.x * dir.x + dir.y * dir.y);
        gl_FragColor = vec4(1., 1., 1., 1. - dst);
    }"#;

    pub const META: ShaderMeta = ShaderMeta {
        images: &[],
        uniforms: UniformBlockLayout {
            uniforms: &[
                UniformDesc::new("projection", UniformType::Mat4),
                UniformDesc::new("light", UniformType::Float2),
                UniformDesc::new("size", UniformType::Float1),
            ],
        },
    };

    #[repr(C)]
    pub struct Uniforms {
        pub projection: glam::Mat4,
        pub light: Vec2,
        pub size: f32,
    }
}

/// Vertex and Fragment shader to render shadows (used in offscreen pipeline)
mod offscreen_shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec3 normal;
    varying highp vec3 inter_normal;
    
    uniform mat4 projection;

    void main() {
        gl_Position = projection * vec4(pos, 0, 1);
        inter_normal = normal;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying highp vec3 inter_normal;

    uniform highp float th;

    void main() {
        lowp float mid = 0.5 - abs(inter_normal.x / inter_normal.z - 0.5);
        if (mid < th) {
            mid = (th - mid) / th;
        } else {
            mid = 0.;
        }
        gl_FragColor = vec4(1., 1., 1., 1. - mid);
    }"#;

    pub const META: ShaderMeta = ShaderMeta {
        images: &[],
        uniforms: UniformBlockLayout {
            uniforms: &[
                UniformDesc::new("projection", UniformType::Mat4),
                UniformDesc::new("th", UniformType::Float1),
            ],
        },
    };

    #[repr(C)]
    pub struct Uniforms {
        pub projection: glam::Mat4,
        pub th: f32,
    }
}

/// Vertex and Fragment shader to render texture fullscreen
/// (texture comes from offscreen pipeline)
mod display_shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec2 uv0;

    varying lowp vec2 uv;
    varying lowp vec2 fpos;

    // uniform mat4 projection;

    void main() {
        gl_Position = vec4(pos, 0, 1);
        uv = uv0;
        fpos = pos;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 uv;
    varying lowp vec2 fpos;

    uniform sampler2D tex;

    void main() {
        lowp vec4 color = texture2D(tex, uv);
        gl_FragColor = vec4(vec3(1., 1., 0.5) - color.rgb, color.r);
    }"#;

    pub const META: ShaderMeta = ShaderMeta {
        images: &["tex"],
        uniforms: UniformBlockLayout { uniforms: &[] },
    };
}
