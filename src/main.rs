use graphics_tutorial1_lib::{Vertex, State, Instance};
use winit::{event::{ElementState, Event, KeyEvent, WindowEvent}, event_loop::EventLoop, keyboard::{KeyCode, PhysicalKey}, window::WindowBuilder};
use cgmath::{self, Zero, Rotation3, InnerSpace};
use arc_parser;

struct Sphere {
    verticies: Vec<Vertex>,
    indices: Vec<u16>,
}
impl Sphere {
    pub fn new() -> Self {
        // Parameters for sphere generation
        let radius = 0.5; // to match scale of existing vertices
        let sectors = 32; // horizontal slices
        let stacks = 16;  // vertical stacks
        
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
                    indices.push((row1 + j) as u16);
                    indices.push((row2 + next_j) as u16);
                    indices.push((row2 + j) as u16);
                } else if i == stacks - 1 { // South pole cap
                    indices.push((row1 + j) as u16);
                    indices.push((row1 + next_j) as u16);
                    indices.push((row2 + j) as u16);
                } else { // Body (quad formed by two triangles)
                    indices.push((row1 + j) as u16);
                    indices.push((row1 + next_j) as u16);
                    indices.push((row2 + j) as u16);
                    
                    indices.push((row1 + next_j) as u16);
                    indices.push((row2 + next_j) as u16);
                    indices.push((row2 + j) as u16);
                }
            }
        }
        
        Self {
            verticies,
            indices,
        }
    }
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run(file: &str) {
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

    // read from input file
    let blocks = arc_parser::parser::parser::read_file(file, true).unwrap().unwrap();
    let sphere = Sphere::new();
    let mut instances: Vec<Instance> = Vec::new();
    // construct instances
    let block = blocks.get(0).unwrap();
    for i in 0..block.atoms.len() {
        let atom = block.atoms.get(i).unwrap();
        let position = cgmath::Vector3 {
            x: atom.coordinate.0 as f32,
            y: atom.coordinate.1 as f32,
            z: atom.coordinate.2 as f32,
        };
        let rotation = if position.is_zero() {
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
        };

        instances.push(Instance {
            position,
            rotation,
        });
    }
    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(&window, &sphere.verticies, &sphere.indices, instances).await;
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

fn main() {
    // get the file name from the first argument passed to the program
    let args: Vec<String> = std::env::args().collect();
    let file = &args[1];
    pollster::block_on(run(&file));
}
