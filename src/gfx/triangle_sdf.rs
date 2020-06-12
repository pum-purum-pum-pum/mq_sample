use glam::{vec2, Mat4, Vec2};
use miniquad::*;

#[repr(C)]
struct Vertex {
    pos: Vec2,
}

pub struct TriangleSDF {
    display_pipeline: Pipeline,
    display_bindings: Bindings,
    /// Threshold for smoothstep in fragment shader
    pub sdf_edge: f32,
}

impl TriangleSDF {
    pub fn new(ctx: &mut Context) -> TriangleSDF {
        // Render huge recrangle, fragment shader do the rest
        let size = 5.;
        let vpos = [
            vec2(-size, -size),
            vec2(size, -size),
            vec2(size, size),
            vec2(-size, size),
        ];
        let mut vertices = vec![];
        for v in vpos.iter() {
            vertices.push(Vertex { pos: *v });
        }
        let display_vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);

        #[rustfmt::skip]
        let indices: &[u16] = &[0, 1, 2, 2, 3, 0];
        let display_index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);
        let display_bindings = Bindings {
            vertex_buffers: vec![display_vertex_buffer],
            index_buffer: display_index_buffer,
            images: vec![],
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
            &[VertexAttribute::new("pos", VertexFormat::Float2)],
            default_shader,
            PipelineParams {
                depth_write: false,
                ..Default::default()
            },
        );

        TriangleSDF {
            display_pipeline,
            display_bindings,
            sdf_edge: 0.01,
        }
    }

    pub fn draw(&mut self, ctx: &mut Context, projection: Mat4) {
        ctx.apply_pipeline(&self.display_pipeline);
        ctx.apply_bindings(&self.display_bindings);
        ctx.apply_uniforms(&display_shader::Uniforms {
            projection,
            sdf_edge: self.sdf_edge,
        });
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();
    }
}

mod display_shader {
    use miniquad::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    varying highp vec2 fpos;
    
    uniform mat4 projection;

    void main() {
        gl_Position = projection * (vec4(pos, 0, 1) + vec4(1., -1., 0., 0.));
        fpos = pos;
    }"#;

    // https://www.shadertoy.com/view/Xl2yDW
    pub const FRAGMENT: &str = r#"#version 100
    varying highp vec2 fpos;
    uniform lowp float sdf_edge;

    lowp float sdEquilateralTriangle(  in lowp vec2 p ) {
        const lowp float k = sqrt(3.0);
        p.x = abs(p.x) - 1.0;
        p.y = p.y + 1.0/k;
        if( p.x+k*p.y>0.0 ) p=vec2(p.x-k*p.y,-k*p.x-p.y)/2.0;
        p.x -= clamp( p.x, -2.0, 0.0 );
        return -length(p)*sign(p.y);
    }

    void main() {
        lowp vec2 p = fpos;
        p *= 2.0;
        
        lowp float d = sdEquilateralTriangle( p );
        lowp float edge1 = 0.;
        lowp float edge2 = sdf_edge;
        lowp float res = 0.;
        if (d > edge1) {
            res = smoothstep(edge1, edge2, d);
        } else {
            res = 0.;
        };
        
        gl_FragColor = vec4(1., 1., 1., 1. - res);
        // gl_FragColor = vec4(fpos.x, fpos.y, 0., res);
    }"#;

    pub const META: ShaderMeta = ShaderMeta {
        images: &[],
        uniforms: UniformBlockLayout {
            uniforms: &[
                UniformDesc::new("projection", UniformType::Mat4),
                UniformDesc::new("sdf_edge", UniformType::Float1),
            ],
        },
    };

    #[repr(C)]
    pub struct Uniforms {
        pub projection: glam::Mat4,
        pub sdf_edge: f32,
    }
}
