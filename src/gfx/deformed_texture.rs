use crate::polygon::projective_textures;
use glam::{vec2, Mat4, Vec2, Vec3};
use miniquad::*;
use png;

const MAX_TEXTURE_VERTICES: usize = 1000;

#[repr(C)]
struct TexVertex {
    pos: Vec2,
    uv: Vec3,
}

pub struct TextureRenderer {
    display_pipeline: Pipeline,
    display_bindings: Bindings,
    vertices: Vec<TexVertex>,
    vertex_buffer: Buffer,
    pub time: f32,
}

impl TextureRenderer {

    /// Update rendering pipeline with new shadows
    pub fn deform_texture(&mut self, ctx: &mut Context) {
        // self.time = (self.time + 0.01) % (2. * std::f32::consts::PI);
        self.vertices.clear();
        let size = 1.;
        self.vertices.shrink_to_fit();
        let vpos = [
            vec2(-size, -size),
            vec2(size, -size),
            vec2(size, f32::sin(self.time) * size),
            vec2(-1.5 * size, size),
        ];
        let uv = [vec2(1., 1.), vec2(0., 1.), vec2(0., 0.), vec2(1., 0.)];
        let uv = projective_textures(&vpos, &uv);
        let mut vertices = vec![];
        for (v, tex) in vpos.iter().zip(uv.iter()) {
            vertices.push(TexVertex { pos: *v, uv: *tex });
        }
        self.vertices = vertices;
        // vertex_buffer
        self.vertex_buffer.update(ctx, &self.vertices);
    }

    pub fn new(ctx: &mut Context) -> TextureRenderer {
        // display pipeline
        let size = 1.;
        let vpos = [
            vec2(-size, -size),
            vec2(size, -size),
            vec2(size, 2. * size),
            vec2(-1.5 * size, size),
        ];
        let uv = [vec2(1., 1.), vec2(0., 1.), vec2(0., 0.), vec2(1., 0.)];
        let uv = projective_textures(&vpos, &uv);
        let mut vertices = vec![];
        for (v, tex) in vpos.iter().zip(uv.iter()) {
            vertices.push(TexVertex { pos: *v, uv: *tex });
        }
        // let display_vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);
        let display_vertex_buffer = Buffer::stream(ctx, BufferType::VertexBuffer, MAX_TEXTURE_VERTICES);

        let tiger = include_bytes!("../../vintage-robot.png");
        let decoder = png::Decoder::new(&tiger[..]);
        let (info, mut reader) = decoder.read_info().unwrap();
        let mut img_data = vec![0; info.buffer_size()];
        reader.next_frame(&mut img_data).unwrap();
        let texture = Texture::from_rgba8(ctx, info.width as u16, info.height as u16, &img_data);

        #[rustfmt::skip]
        let indices: &[u16] = &[0, 1, 2, 3, 2, 0];
        let display_index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);
        let display_bindings = Bindings {
            vertex_buffers: vec![display_vertex_buffer],
            index_buffer: display_index_buffer,
            images: vec![texture],
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
                VertexAttribute::new("uv0", VertexFormat::Float3),
            ],
            default_shader,
            PipelineParams {
                depth_write: false,
                ..Default::default()
            },
        );

        TextureRenderer {
            display_pipeline,
            display_bindings,
            vertices: vec![],
            vertex_buffer: display_vertex_buffer,
            time: 0.
        }
    }

    pub fn draw(&mut self, ctx: &mut Context, projection: Mat4) {
        ctx.apply_pipeline(&self.display_pipeline);
        ctx.apply_bindings(&self.display_bindings);
        ctx.apply_uniforms(&display_shader::Uniforms { projection });
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();
    }
}

mod display_shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec3 uv0;
    varying highp vec3 uvq;
    
    uniform mat4 projection;

    void main() {
        gl_Position = projection * vec4(pos, 0, 1);
        uvq = uv0;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying highp vec3 uvq;
    uniform sampler2D tex;

    void main() {
        gl_FragColor = texture2D(tex, uvq.xy / uvq.z);
    }"#;

    pub const META: ShaderMeta = ShaderMeta {
        images: &["tex"],
        uniforms: UniformBlockLayout {
            uniforms: &[UniformDesc::new("projection", UniformType::Mat4)],
        },
    };

    #[repr(C)]
    pub struct Uniforms {
        pub projection: glam::Mat4,
    }
}
