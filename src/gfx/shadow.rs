use crate::polygon::projective_textures;
use glam::{vec2, vec3, Mat4, Vec2, Vec3};
use miniquad::*;

const MAX_SHADOW_VERTICES: usize = 1000;
const MAX_SHADOW_INDICES: usize = 1500;

#[repr(C)]
struct ShadowVertex {
    pos: Vec2,
    normal: Vec3,
}

pub struct ShadowRenderer {
    pipeline: Pipeline,
    bindings: Bindings,
    pub shadows: Vec<[Vec2; 4]>,
    vertices: Vec<ShadowVertex>,
    indices: Vec<u16>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
}

impl ShadowRenderer {
    pub fn push_shadow(&mut self, shadow: [Vec2; 4]) {
        self.shadows.push(shadow);
    }

    pub fn clear_shadows_buffer(&mut self) {
        self.shadows.clear()
    }

    pub fn reconstruct_buffers(&mut self, ctx: &mut Context) {
        // possible to shrink here if memory is important
        self.vertices.clear();
        self.indices.clear();
        self.vertices.shrink_to_fit();
        self.indices.shrink_to_fit();
        let normals = [vec2(1., 0.), vec2(1., 1.), vec2(0., 1.), vec2(0., 0.)];
        for shadow in self.shadows.iter() {
            let newi = vec![0, 1, 2, 3, 2, 0];
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
        { // offscreen pipeline
            
        }

        { // display pipeline

        }

        let vertex_buffer = Buffer::stream(ctx, BufferType::VertexBuffer, MAX_SHADOW_VERTICES);
        let index_buffer = Buffer::stream(ctx, BufferType::IndexBuffer, MAX_SHADOW_INDICES);

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer: index_buffer,
            images: vec![],
        };

        let shader = Shader::new(ctx, shader::VERTEX, shader::FRAGMENT, shader::META).unwrap();
        let pipeline = Pipeline::new(
            ctx,
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("normal", VertexFormat::Float3),
            ],
            shader,
        );

        ShadowRenderer {
            pipeline,
            bindings,
            vertex_buffer,
            index_buffer,
            shadows: vec![],
            vertices: vec![],
            indices: vec![],
        }
    }

    pub fn draw(&mut self, ctx: &mut Context, projection: Mat4) {
        ctx.apply_pipeline(&self.pipeline);
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms { projection });
        ctx.draw(0, self.indices.len() as i32, 1);
    }
}

mod shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec3 normal;
    varying lowp vec3 inter_normal;
    
    uniform mat4 projection;

    void main() {
        gl_Position = projection * vec4(pos, 0, 1);
        inter_normal = normal;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec3 inter_normal;

    void main() {
        lowp float th = 0.05;
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
            uniforms: &[UniformDesc::new("projection", UniformType::Mat4)],
        },
    };

    #[repr(C)]
    pub struct Uniforms {
        pub projection: glam::Mat4,
    }
}
