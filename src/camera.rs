use bytemuck::{Pod, Zeroable};
use cgmath::{Rotation, Rotation3};
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
    is_keyboard_left_pressed: bool,
    is_keyboard_right_pressed: bool,
    is_keyboard_up_pressed: bool,
    is_keyboard_down_pressed: bool,
    is_reset_pressed: bool,
    is_keyboard_ctrl_pressed: bool,
    // mouse related fields
    is_mouse_left_pressed: bool,
    is_mouse_right_pressed: bool,
    is_mouse_middle_pressed: bool,
    mouse_movement_sensitivity: f32,
    mouse_movement: Option<(f32, f32)>,
    mouse_wheel_sensitivity: f32,
    mouse_wheel_movement: Option<f32>,
}
impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_keyboard_left_pressed: false,
            is_keyboard_right_pressed: false,
            is_keyboard_up_pressed: false,
            is_keyboard_down_pressed: false,
            is_reset_pressed: false,
            is_keyboard_ctrl_pressed: false,
            // mouse related fields
            mouse_movement_sensitivity: 0.01,
            is_mouse_left_pressed: false,
            is_mouse_right_pressed: false,
            is_mouse_middle_pressed: false,
            mouse_movement: None,
            mouse_wheel_sensitivity: 0.1,
            mouse_wheel_movement: None,
        }
    }

    pub fn process_window_events(&mut self, event: &WindowEvent) -> bool {
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
                match keycode {
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_keyboard_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_keyboard_right_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_keyboard_up_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_keyboard_down_pressed = is_pressed;
                        true
                    }
                    KeyCode::Space => {
                        self.is_reset_pressed = is_pressed;
                        true
                    }
                    KeyCode::ControlLeft | KeyCode::ControlRight => {
                        self.is_keyboard_ctrl_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            },
            WindowEvent::MouseInput { 
                state,
                button,
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        self.is_mouse_left_pressed = is_pressed;
                        true
                    }
                    MouseButton::Right => {
                        self.is_mouse_right_pressed = is_pressed;
                        true
                    }
                    MouseButton::Middle => {
                        self.is_mouse_middle_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            },
            WindowEvent::MouseWheel { 
                delta,
                ..
            } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        self.mouse_wheel_movement = Some(*y);
                        true
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        self.mouse_wheel_movement = Some(pos.y as f32);
                        true
                    }
                }
            },
            _ => false,
        }
    }

    pub fn process_device_events(&mut self, event: &DeviceEvent) -> bool {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.mouse_movement = Some((delta.0 as f32, delta.1 as f32));
                true
            },
            _ => false,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        // a vector that points from the camera's eye to the camera's target
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        // the distance between the camera's eye and target
        let forward_mag = forward.magnitude();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_keyboard_up_pressed && self.is_keyboard_ctrl_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        } 
        if self.is_keyboard_down_pressed && self.is_keyboard_ctrl_pressed {
            camera.eye -= forward_norm * self.speed;
        }
        // the vector pointing to the right direction of the camera
        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_keyboard_right_pressed && !self.is_keyboard_ctrl_pressed {
            // Rescale the distance between the target and the eye so 
            // that it doesn't change. The eye, therefore, still 
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_keyboard_left_pressed && !self.is_keyboard_ctrl_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
        if self.is_keyboard_up_pressed && !self.is_keyboard_ctrl_pressed {
            let radian = self.speed / forward_mag;
            let axis = forward_norm.cross(camera.up).normalize();
            let new_forward_norm = forward_norm * radian.cos() + axis.cross(forward_norm) * radian.sin();
            let new_forward = new_forward_norm * forward_mag;
            camera.eye = camera.target - new_forward;
            // also update the up vector
            camera.up = cgmath::Quaternion::from_axis_angle(axis, cgmath::Rad(radian)).rotate_vector(camera.up);
        }
        if self.is_keyboard_down_pressed && !self.is_keyboard_ctrl_pressed {
            let radian = - self.speed / forward_mag;
            let axis = forward_norm.cross(camera.up).normalize();
            let new_forward_norm = forward_norm * radian.cos() + axis.cross(forward_norm) * radian.sin();
            let new_forward = new_forward_norm * forward_mag;
            camera.eye = camera.target - new_forward;
            // also update the up vector
            camera.up = cgmath::Quaternion::from_axis_angle(axis, cgmath::Rad(radian)).rotate_vector(camera.up);
        }
        // deal with mouse movement
        if self.is_mouse_right_pressed {
            // only process mouse movement when the right mouse button is pressed
            if let Some((dx, dy)) = self.mouse_movement {
                let dx = dx * self.mouse_movement_sensitivity;
                let dy = dy * self.mouse_movement_sensitivity;
                // rotate the forward vector around the up vector
                let forward = camera.target - camera.eye;
                let forward_norm = forward.normalize();
                let axis = forward_norm.cross(camera.up).normalize();
                // rotate vertically
                let radian = -dy ;// when moving mouse up, we actually want the camera to go down
                let new_forward_norm = forward_norm * radian.cos() + axis.cross(forward_norm) * radian.sin();
                let new_forward = new_forward_norm * forward.magnitude();
                camera.eye = camera.target - new_forward;
                camera.up = cgmath::Quaternion::from_axis_angle(axis, cgmath::Rad(radian)).rotate_vector(camera.up);
                // rotate horizontally
                let right = new_forward_norm.cross(camera.up);
                camera.eye = camera.target - (new_forward_norm + right * dx).normalize() * forward.magnitude();
                self.mouse_movement = None; // reset the mouse movement
            }
        }
        // deal with mouse wheel movement
        if let Some(dy) = self.mouse_wheel_movement {
            // zoom in/out
            let forward = camera.target - camera.eye;
            let forward_norm = forward.normalize();
            let new_forward = forward + forward_norm * dy * self.mouse_wheel_sensitivity;
            camera.eye = camera.target - new_forward;
        }
    }
}