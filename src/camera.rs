use bytemuck::{Pod, Zeroable};
use winit::{
    event::*,
    keyboard::{KeyCode, PhysicalKey},
};

pub struct Camera {
    pub eye: cgmath::Point3<f32>, // coordinates of the camera in world space
    pub target: cgmath::Point3<f32>, // where the camera is looking at in world space
    pub up: cgmath::Vector3<f32>, // the up vector of the camera in world space
    pub aspect: f32, // aspect ratio of the camera
    pub fovy: f32, // vertical field of view in degrees
    pub znear: f32, // near plane
    pub zfar: f32, // far plane
}
impl Camera {
    #[rustfmt::skip]
    const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.5,
        0.0, 0.0, 0.0, 1.0,
    );
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // view is a matrix, which transforms coordinates from world space to view space
        // (a, b, c)^T in word space -> view * (a, b, c)^T in view space
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // proj is a matrix, which transforms coordinates from view space to clip space
        // (a, b, c)^T in view space -> proj * (a, b, c, 1)^T in clip space
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        // return the combined matrix
        // this matrix is used to transform coordinates from world space to clip space
        // (a, b, c)^T in word space -> 
        //  view * (a, b, c, 1)^T in view space -> 
        //  proj * view (a, b, c, 1)^T in clip space(OpenGL clip space) -> 
        //  wgpu clip space
        return Self::OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}
impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}
impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        // a vector that points from the camera's eye to the camera's target
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        // the distance between the camera's eye and target
        let forward_mag = forward.magnitude();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }
        // the vector pointing to the right direction of the camera
        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so 
            // that it doesn't change. The eye, therefore, still 
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}