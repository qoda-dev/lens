use crate::{camera, light, texture};
use std::ops::Range;
use std::path::Path;
use tobj::*;
use wgpu::util::DeviceExt;

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
    pub normal: [[f32; 3]; 3],
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in
                // the shader.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub material_layout: wgpu::BindGroupLayout,
}

pub struct Mesh {
    pub geometry: Geometry,
    pub material: usize,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Geometry {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

impl Model {
    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
    ) -> Result<Self, ()> {
        let (obj_models, obj_materials) = tobj::load_obj(
            path.as_ref(),
            &LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
        )
        .unwrap();

        let obj_materials = obj_materials.unwrap();

        // We're assuming that the texture files are stored with the obj file
        let containing_folder = path.as_ref().parent().unwrap();

        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        // This is only for TextureSampleType::Depth
                        comparison: false,
                        // This should be true if the sample_type of the texture is:
                        //     TextureSampleType::Float { filterable: true }
                        // Otherwise you'll get an error.
                        filtering: true,
                    },
                    count: None,
                },
            ],
            label: Some("material_bind_group_layout"),
        });

        let mut materials = Vec::new();
        for mat in obj_materials {
            let diffuse_path = mat.diffuse_texture;
            let diffuse_texture =
                texture::Texture::load(device, queue, containing_folder.join(diffuse_path))
                    .unwrap();

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &material_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    },
                ],
                label: None,
            });

            materials.push(Material {
                name: mat.name,
                diffuse_texture,
                bind_group,
            });
        }

        let mut meshes = Vec::new();
        for m in obj_models {
            let mut vertices = Vec::new();
            for i in 0..m.mesh.positions.len() / 3 {
                vertices.push(ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                });
            }

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", path.as_ref())),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", path.as_ref())),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            let geometry = Geometry {
                name: m.name,
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
            };

            meshes.push(Mesh {
                geometry: geometry,
                material: m.mesh.material_id.unwrap_or(0),
            });
        }

        Ok(Self {
            meshes,
            materials,
            material_layout,
        })
    }
}

pub trait Renderer {
    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader: wgpu::ShaderModuleDescriptor,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(&shader);

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLAMPING
                clamp_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        })
    }
}

pub struct ModelRenderer {
    pub model: Model,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl Renderer for ModelRenderer {}

impl ModelRenderer {
    pub fn new_renderer(
        model: Model,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera: &camera::Camera,
        light: &light::Light,
        shader_file: std::borrow::Cow<str>,
    ) -> ModelRenderer {
        let render_pipeline = {
            let bind_group_layouts = &[
                &model.material_layout,
                &camera.bind_group_layout,
                &light.bind_group_layout,
            ];
            let render_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: bind_group_layouts,
                    push_constant_ranges: &[],
                });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_file),
            };
            ModelRenderer::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[ModelVertex::desc(), InstanceRaw::desc()],
                shader,
            )
        };

        ModelRenderer {
            model: model,
            render_pipeline: render_pipeline,
        }
    }
}

pub struct MeshRenderer {
    pub render_pipeline: wgpu::RenderPipeline,
}
impl Renderer for MeshRenderer {}

impl MeshRenderer {
    pub fn new_renderer(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        camera: &camera::Camera,
        light: &light::Light,
        shader_file: std::borrow::Cow<str>,
    ) -> MeshRenderer {
        let render_pipeline = {
            let bind_group_layouts = &[&camera.bind_group_layout, &light.bind_group_layout];
            let render_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: bind_group_layouts,
                    push_constant_ranges: &[],
                });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_file),
            };
            ModelRenderer::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[ModelVertex::desc(), InstanceRaw::desc()],
                shader,
            )
        };

        MeshRenderer {
            render_pipeline: render_pipeline,
        }
    }
}

pub trait DrawModel<'a> {
    fn draw_model(&mut self, model: &'a Model, bind_groups: &'a [&'a wgpu::BindGroup]);

    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        bind_groups: &'a [&'a wgpu::BindGroup],
    );

    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material_bind_group: Option<&'a wgpu::BindGroup>,
        bind_groups: &'a [&'a wgpu::BindGroup],
    );

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material_bind_group: Option<&'a wgpu::BindGroup>,
        instances: Range<u32>,
        bind_groups: &'a [&'a wgpu::BindGroup],
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    // draw a complete model
    fn draw_model(&mut self, model: &'b Model, bind_groups: &'b [&'b wgpu::BindGroup]) {
        self.draw_model_instanced(model, 0..1, bind_groups);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        bind_groups: &'b [&'b wgpu::BindGroup],
    ) {
        for mesh in &model.meshes {
            let material_bind_group = &model.materials[mesh.material].bind_group;
            self.draw_mesh_instanced(
                mesh,
                Some(material_bind_group),
                instances.clone(),
                bind_groups,
            );
        }
    }

    // draw each mesh in a model
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material_bind_group: Option<&'b wgpu::BindGroup>,
        bind_groups: &'b [&'b wgpu::BindGroup],
    ) {
        self.draw_mesh_instanced(mesh, material_bind_group, 0..1, bind_groups);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material_bind_group: Option<&'b wgpu::BindGroup>,
        instances: Range<u32>,
        bind_groups: &'b [&'b wgpu::BindGroup],
    ) {
        // set vertex & index buffer
        self.set_vertex_buffer(0, mesh.geometry.vertex_buffer.slice(..));
        self.set_index_buffer(
            mesh.geometry.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        // set material bind group
        let mut offset = 0;
        if let Some(material) = material_bind_group {
            self.set_bind_group(0, material, &[]);
            offset += 1;
        }

        // set light bind group
        bind_groups.iter().enumerate().for_each(|(index, group)| {
            self.set_bind_group(index as u32 + offset, *group, &[]);
        });

        // draw the mesh
        self.draw_indexed(0..mesh.geometry.num_elements, 0, instances);
    }
}
