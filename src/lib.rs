use std::iter;
use bytemuck::{Pod, Zeroable};
use arc_parser;
mod texture;
mod camera;

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{WindowBuilder, Window},
};
use cgmath::prelude::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}
impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
    ];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    scale: cgmath::Vector3<f32>,
    tex_offset: cgmath::Vector2<f32>,
    tex_scale: cgmath::Vector2<f32>,
}
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    tex_offset: [f32; 2],
    tex_scale: [f32; 2],
}
impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        let scale = cgmath::Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        let model = cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation) * scale;
        InstanceRaw {
            model: model.into(),
            tex_offset: self.tex_offset.into(),
            tex_scale: self.tex_scale.into(),
        }
    }
}
impl InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5, not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
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
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 18]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ]
        }
    }
}

// Sphere generation (used for atoms)
struct Sphere {
    verticies: Vec<Vertex>,
    indices: Vec<u32>,
}
impl Sphere {
    pub fn new(radius: f32, sectors: u32, stacks: u32) -> Self {
        
        let mut verticies = Vec::new();
        let mut indices = Vec::new();
        
        // Generate vertices
        for i in 0..=stacks {
            let phi = std::f32::consts::PI * i as f32 / stacks as f32;
            let y = radius * phi.cos();
            let r = radius * phi.sin(); // radius at this stack
            
            for j in 0..sectors {
                let theta = 2.0 * std::f32::consts::PI * j as f32 / sectors as f32;
                
                // Vertex position
                let x = r * theta.cos();
                let z = r * theta.sin();
                
                // Texture coordinates
                let u = j as f32 / sectors as f32;
                let v = i as f32 / stacks as f32;
                
                verticies.push(Vertex {
                    position: [x, y, z],
                    tex_coords: [u, v],
                });
            }
        }
        
        // Generate indices
        for i in 0..stacks {
            let row1 = i * sectors;
            let row2 = (i + 1) * sectors;
            
            for j in 0..sectors {
                let next_j = (j + 1) % sectors;
                
                if i == 0 { // North pole cap
                    indices.push((row1 + j) as u32);
                    indices.push((row2 + next_j) as u32);
                    indices.push((row2 + j) as u32);
                } else if i == stacks - 1 { // South pole cap
                    indices.push((row1 + j) as u32);
                    indices.push((row1 + next_j) as u32);
                    indices.push((row2 + j) as u32);
                } else { // Body (quad formed by two triangles)
                    indices.push((row1 + j) as u32);
                    indices.push((row1 + next_j) as u32);
                    indices.push((row2 + j) as u32);
                    
                    indices.push((row1 + next_j) as u32);
                    indices.push((row2 + next_j) as u32);
                    indices.push((row2 + j) as u32);
                }
            }
        }
        
        Self {
            verticies,
            indices,
        }
    }
}
// Cylinder generation (used for bonds)
struct Cylinder {
    verticies: Vec<Vertex>,
    indices: Vec<u32>,
}
impl Cylinder {
    fn new(radius: f32, height: f32, segments: u32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // Center points for top and bottom faces
        let top_center = vertices.len() as u32;
        vertices.push(Vertex {
            position: [0.0, height/2.0, 0.0],
            tex_coords: [0.5, 0.5],
        });
        
        let bottom_center = vertices.len() as u32;
        vertices.push(Vertex {
            position: [0.0, -height/2.0, 0.0],
            tex_coords: [0.5, 0.5],
        });
        
        // Add vertices for the circles at top and bottom
        let top_start_idx = vertices.len() as u32;
        for i in 0..segments {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let x = radius * theta.cos();
            let z = radius * theta.sin();
            
            // Texture coordinates
            let u = i as f32 / segments as f32;
            
            // Top circle vertex
            vertices.push(Vertex {
                position: [x, height/2.0, z],
                tex_coords: [u, 0.0],
            });
        }
        
        // Bottom circle vertices
        let bottom_start_idx = vertices.len() as u32;
        for i in 0..segments {
            let theta = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
            let x = radius * theta.cos();
            let z = radius * theta.sin();
            
            // Texture coordinates
            let u = i as f32 / segments as f32;
            
            // Bottom circle vertex
            vertices.push(Vertex {
                position: [x, -height/2.0, z],
                tex_coords: [u, 1.0],
            });
        }
        
        // Generate indices for triangles
        for i in 0..segments {
            let next_i = (i + 1) % segments;
            
            // Top face
            indices.push(top_center);
            indices.push(top_start_idx + i);
            indices.push(top_start_idx + next_i);
            
            // Bottom face (reversed winding)
            indices.push(bottom_center);
            indices.push(bottom_start_idx + next_i);
            indices.push(bottom_start_idx + i);
            
            // Side quads (two triangles per quad)
            let top_current = top_start_idx + i;
            let top_next = top_start_idx + next_i;
            let bottom_current = bottom_start_idx + i;
            let bottom_next = bottom_start_idx + next_i;
            
            // First triangle
            indices.push(top_current);
            indices.push(bottom_current);
            indices.push(top_next);
            
            // Second triangle
            indices.push(bottom_current);
            indices.push(bottom_next);
            indices.push(top_next);
        }
        
        Self {
            verticies: vertices,
            indices,
        }
    }
}

// a wrapper for all objects in the scene
struct RenderObject {
    vertex_buffer: wgpu::Buffer,
    num_verticies: u32,
    index_buffer: wgpu::Buffer,
    num_indicies: u32,
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    object_type: ObjectType,
}
// types of objects in the scene
#[derive(Debug, Clone, Copy, PartialEq)]
enum ObjectType {
    Sphere,
    Cylinder,
}
impl RenderObject {
    fn new(
        device: &wgpu::Device,
        verticies: &[Vertex],
        indicies: &[u32],
        instances: Vec<Instance>,
        object_type: ObjectType,
    ) -> Self {
        // vertex buffer
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", object_type)),
                contents: bytemuck::cast_slice(verticies),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        // index buffer
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", object_type)),
                contents: bytemuck::cast_slice(indicies),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        // instance buffer
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Instance Buffer", object_type)),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        Self {
            vertex_buffer,
            num_verticies: verticies.len() as u32,
            index_buffer,
            num_indicies: indicies.len() as u32,
            instances,
            instance_buffer,
            object_type,
        }
    }

    fn update_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: Vec<Instance>,
    ) {
        self.instances = instances;
        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        
        if instance_data.len() * std::mem::size_of::<Instance>() <= self.instance_buffer.size() as usize {
            // if the current buffer is large enough, reuse the current buffer
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&instance_data),
            );
        } else {
            // if the current buffer is not large enough, create a new buffer
            self.instance_buffer = device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Instance Buffer", self.object_type)),
                    contents: bytemuck::cast_slice(&instance_data),
                    usage: wgpu::BufferUsages::VERTEX,
                }
            );
        }
    }
}
pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    clear_color: wgpu::Color,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline,
    diffuse_bind_group: wgpu::BindGroup,
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: camera::CameraController,
    depth_texture: texture::Texture,
    render_objects: Vec<RenderObject>,
}

impl<'a> State<'a> {

    pub async fn from_structure_block(window: &'a Window, block: &arc_parser::modules::structures::StructureBlock) -> State<'a> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: Default::default(),
                },
                // Some(&std::path::Path::new("trace")), // Trace path
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };
        let clear_color = wgpu::Color::BLACK;
        // load the texture
        let diffuse_bytes = include_bytes!("texture.png");
        let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "texture.png").unwrap();
        // initiate texture bind group
        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bindgroup_layout")
            }
        );
        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout, 
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
                label: Some("diffuse_bind_group"),
            }
        );
        // initiate camera
        let camera = camera::Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 1.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera); // camera_uniform.view_proj stores the view projection matrix
        // load view projection matrix to uniform buffer
        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );
        // initiate camera bind group
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        // initiate sphere for atoms
        let sphere = Sphere::new(1.0, 32, 16);
        let mut sphere_instances: Vec<Instance> = Vec::new();
        // initiate cylinder for bonds
        let cylinder = Cylinder::new(1.0, 1.0, 16);
        let mut cylinder_instances: Vec<Instance> = Vec::new();
        // calcualte the center of cell
        let center_of_cell = cgmath::Vector3 {
            x: block.crystal.x as f32 / 2.0,
            y: block.crystal.y as f32 / 2.0,
            z: block.crystal.z as f32 / 2.0,
        };
        // initiate atom instances
        for i in 0..block.atoms.len() {
            let atom = block.atoms.get(i).unwrap();
            let position = cgmath::Vector3 {
                x: atom.coordinate.0 as f32,
                y: atom.coordinate.1 as f32,
                z: atom.coordinate.2 as f32,
            } - center_of_cell;
            let rotation = if position.is_zero() {
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            } else {
                cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
            };
            let (tex_offset, tex_scale) = match atom.element.as_str() {
                "O" => ([7.0/11.0, 0.0], [1.0/11.0, 1.0/11.0]),
                "Si" => ([2.0/11.0, 1.0/11.0], [1.0/11.0, 1.0/11.0]),
                _ => ([0.0, 0.0], [1.0/11.0, 1.0/11.0]),
            };
            let sphere_radius = match atom.element.as_str() {
                "O" => 0.4,
                "Si" => 0.5,
                _ => 0.3,
            };
            let scale = cgmath::Vector3 {
                x: sphere_radius,
                y: sphere_radius,
                z: sphere_radius,
            };
            sphere_instances.push(Instance {
                position,
                rotation,
                scale,
                tex_offset: tex_offset.into(),
                tex_scale: tex_scale.into(),
            });
        }
        // initiate bond instances
        let bond_matrix = arc_parser::analyzer::arc_analyzer::calc_coordination_matrix(block);
        for i in 0..bond_matrix.ncols() {
            for j in i..bond_matrix.nrows() {
                if bond_matrix[(i, j)] == 1 {
                    let start_atom = block.atoms.get(i).unwrap();
                    let end_atom = block.atoms.get(j).unwrap();
                    let start_position = cgmath::Vector3 {
                        x: start_atom.coordinate.0 as f32,
                        y: start_atom.coordinate.1 as f32,
                        z: start_atom.coordinate.2 as f32,
                    } - center_of_cell;
                    let end_position = cgmath::Vector3 {
                        x: end_atom.coordinate.0 as f32,
                        y: end_atom.coordinate.1 as f32,
                        z: end_atom.coordinate.2 as f32,
                    } - center_of_cell;
                    let bond_position = (start_position + end_position) / 2.0;
                    let bond_direction = end_position - start_position;
                    // Calculate bond length
                    let bond_length = bond_direction.magnitude();

                    let radius_scale = 0.1;
                    let height_scale = bond_length;
                    let scale = cgmath::Vector3 {
                        x: radius_scale,
                        y: height_scale,
                        z: radius_scale,
                    };

                    // Default cylinder orientation is along y-axis
                    let cylinder_default_direction = cgmath::Vector3::unit_y();

                    // Calculate the rotation needed to align the cylinder with the bond direction
                    let normalized_bond_direction = bond_direction.normalize();

                    // Find the rotation axis (cross product of default direction and target direction)
                    let rotation_axis = cylinder_default_direction.cross(normalized_bond_direction);

                    // Handle special cases (parallel vectors)
                    let rotation = if rotation_axis.magnitude() < 1e-6 {
                        // If bond is pointing down, rotate 180 degrees around x-axis
                        if normalized_bond_direction.dot(cylinder_default_direction) < 0.0 {
                            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_x(), cgmath::Deg(180.0))
                        } else {
                            // No rotation needed if already aligned
                            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_x(), cgmath::Deg(0.0))
                        }
                    } else {
                        // Calculate angle between the two directions
                        let angle = cylinder_default_direction.dot(normalized_bond_direction)
                            .clamp(-1.0, 1.0)
                            .acos();

                        // Create quaternion for the rotation
                        cgmath::Quaternion::from_axis_angle(rotation_axis.normalize(), cgmath::Rad(angle))
                    };
                    let (tex_offset, tex_scale) = ([0.0, 0.0], [1.0, 1.0]);
                    cylinder_instances.push(Instance {
                        position: bond_position,
                        rotation,
                        scale,
                        tex_offset: tex_offset.into(),
                        tex_scale: tex_scale.into(),
                    });
                }
            }
        }
        // initiate cell bondaries (use thin sylinders)
        let crystal_size = cgmath::Vector3 {
            x: block.crystal.x as f32,
            y: block.crystal.y as f32,
            z: block.crystal.z as f32,
        };
        let vertices = [
            [-crystal_size.x/2.0, -crystal_size.y/2.0, -crystal_size.z/2.0], // 0
            [ crystal_size.x/2.0, -crystal_size.y/2.0, -crystal_size.z/2.0], // 1
            [ crystal_size.x/2.0,  crystal_size.y/2.0, -crystal_size.z/2.0], // 2
            [-crystal_size.x/2.0,  crystal_size.y/2.0, -crystal_size.z/2.0], // 3
            [-crystal_size.x/2.0, -crystal_size.y/2.0,  crystal_size.z/2.0], // 4
            [ crystal_size.x/2.0, -crystal_size.y/2.0,  crystal_size.z/2.0], // 5
            [ crystal_size.x/2.0,  crystal_size.y/2.0,  crystal_size.z/2.0], // 6
            [-crystal_size.x/2.0,  crystal_size.y/2.0,  crystal_size.z/2.0], // 7
        ];
        let edges = [
            [0, 1], [1, 2], [2, 3], [3, 0],  // bottom
            [4, 5], [5, 6], [6, 7], [7, 4],  // top
            [0, 4], [1, 5], [2, 6], [3, 7],  // vertical
        ];
        let line = Cylinder::new(1.0, 1.0, 32);
        let mut line_instances: Vec<Instance> = Vec::new();
        let line_width = 0.03;
        for edge in &edges {
            let start = cgmath::Vector3 {
                x: vertices[edge[0]][0],
                y: vertices[edge[0]][1],
                z: vertices[edge[0]][2],
            };
            let end = cgmath::Vector3 {
                x: vertices[edge[1]][0],
                y: vertices[edge[1]][1],
                z: vertices[edge[1]][2],
            };
            line_instances.push(
                Instance {
                    position: (start + end) / 2.0,
                    rotation: {
                        let direction = end - start;
                        let normalized_direction = direction.normalize();
                        let default_direction = cgmath::Vector3::unit_y();
                        let rotation_axis = default_direction.cross(normalized_direction);
                        
                        if rotation_axis.magnitude() < 1e-6 {
                            // Vectors are parallel
                            if normalized_direction.dot(default_direction) < 0.0 {
                                // Direction is opposite to y-axis, rotate 180° around x
                                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_x(), cgmath::Deg(180.0))
                            } else {
                                // Direction is same as y-axis, no rotation needed
                                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_x(), cgmath::Deg(0.0))
                            }
                        } else {
                            let angle = default_direction.dot(normalized_direction)
                                .clamp(-1.0, 1.0)
                                .acos();
                            cgmath::Quaternion::from_axis_angle(rotation_axis.normalize(), cgmath::Rad(angle))
                        }
                    },
                    scale: cgmath::Vector3 {
                        x: line_width,
                        y: start.distance(end),
                        z: line_width,
                    },
                    tex_offset: [0.0, 0.0].into(),
                    tex_scale: [1.0, 1.0].into(),
                }
            );
        }

        // initiate a depth texture
        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        // initiate render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            }
        );
        let camera_controller = camera::CameraController::new(0.2);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
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
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            // Useful for optimizing shader compilation on Android
            cache: None,
        });
        let mut render_objects = Vec::new();
        render_objects.push(RenderObject::new(
            &device,
            &sphere.verticies,
            &sphere.indices,
            sphere_instances,
            ObjectType::Sphere,
        ));
        render_objects.push(RenderObject::new(
            &device,
            &cylinder.verticies,
            &cylinder.indices,
            cylinder_instances,
            ObjectType::Cylinder,
        ));
        render_objects.push(RenderObject::new(
            &device,
            &line.verticies,
            &line.indices,
            line_instances,
            ObjectType::Cylinder,
        ));

        Self {
            surface,
            device,
            queue,
            config,
            size,
            clear_color,
            window,
            render_pipeline,
            diffuse_bind_group,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            depth_texture,
            render_objects,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.surface.configure(&self.device, &self.config);
        }
    }

    #[allow(unused_variables)]
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.clear_color = wgpu::Color {
                    r: position.x as f64 / self.size.width as f64,
                    g: position.y as f64 / self.size.height as f64,
                    b: 1.0,
                    a: 1.0,
                };
                true
            }
            _ => self.camera_controller.process_events(event),
        }
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output: wgpu::SurfaceTexture = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, Some(&self.diffuse_bind_group), &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            for object in &self.render_objects {
                render_pass.set_vertex_buffer(0, object.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, object.instance_buffer.slice(..));
                render_pass.set_index_buffer(object.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..object.num_indicies, 0, 0..object.instances.len() as _);
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn show_strucutre(filename: &str) {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");

        let _ = window.request_inner_size(PhysicalSize::new(450, 400));
    }

    // read from file
    let block = arc_parser::parser::parser::read_file(filename, true).unwrap().unwrap();
    let mut state = State::from_structure_block(&window, &block[0]).await;
    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == state.window().id() => {
                    if !state.input(event) {
                        match event {
                            WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                                        ..
                                    },
                                ..
                            } => control_flow.exit(),
                            WindowEvent::Resized(physical_size) => {
                                log::info!("physical_size: {physical_size:?}");
                                surface_configured = true;
                                state.resize(*physical_size);
                            }
                            WindowEvent::RedrawRequested => {
                                // This tells winit that we want another frame after this one
                                state.window().request_redraw();
                                // return if the surface is not configured
                                if !surface_configured {
                                    return;
                                }
                                state.update();
                                match state.render() {
                                    Ok(_) => {}
                                    // Reconfigure the surface if it's lost or outdated
                                    Err(
                                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                    ) => state.resize(state.size),
                                    // The system is out of memory, we should probably quit
                                    Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                                        log::error!("OutOfMemory");
                                        control_flow.exit();
                                    }

                                    // This happens when the a frame takes too long to present
                                    Err(wgpu::SurfaceError::Timeout) => {
                                        log::warn!("Surface timeout")
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        })
        .unwrap();
}