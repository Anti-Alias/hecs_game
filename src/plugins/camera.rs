use std::f32::consts::PI;
use glam::{Mat4, Quat, Vec2, Vec3};
use hecs::World;
use winit::keyboard::KeyCode;
use crate::math::{lerp_matrices, Transform};
use crate::{App, Cursor, Game, Keyboard, Plugin, Rect, RunContext, Stage, Window, WindowRequests};

const SENSITIVITY_SCALE: f32 = 0.005;
const SCROLL_SENSITIVITY_SCALE: f32 = 0.1;

pub struct FlycamPlugin;
impl Plugin for FlycamPlugin {
    fn install(&mut self, app: &mut App) {
        app.add_system(Stage::Update, control_flycams);
        app.add_system(Stage::PostUpdate, set_cam_projections);
    }
}

fn control_flycams(game: &mut Game, ctx: RunContext) {

    let Some(keyboard)  = game.try_get::<&Keyboard>() else { return };
    let Some(cursor)    = game.try_get::<&Cursor>() else { return };
    let mut world       = game.get::<&mut World>();
    let mut requests    = game.get::<&mut WindowRequests>();
    let cursor_movement = cursor.movement();
    let delta           = ctx.delta_secs();

    for (_, (transform, camera, controller)) in world.query_mut::<(&mut Transform, &mut Camera, &mut CameraController)>() {
        
        // Toggles flycam mode.
        if keyboard.is_just_pressed(KeyCode::Escape) {
            match controller.flycam_mode {
                FlycamMode::Enabled => {
                    requests.set_cursor_grab(false);
                    requests.set_cursor_visible(true);
                    controller.flycam_mode = FlycamMode::Disabled;
                },
                FlycamMode::Disabled => {
                    requests.set_cursor_grab(true);
                    requests.set_cursor_visible(false);
                    controller.flycam_mode = FlycamMode::Enabled
                },
                _ => {}
            }
        }
        if controller.flycam_mode != FlycamMode::Enabled { continue }

        // Alters t value based on scroll
        controller.t += cursor.scroll().y * controller.sensitivity * SCROLL_SENSITIVITY_SCALE;
        controller.t = controller.t.max(0.0).min(1.0);

        // Rotates camera
        controller.yaw -= cursor_movement.x * controller.sensitivity * SENSITIVITY_SCALE;
        controller.pitch -= cursor_movement.y * controller.sensitivity * SENSITIVITY_SCALE;
        transform.rotation = controller.rotation();

        // Moves camera
        let (right, up, forward) = controller.axes();
        if keyboard.is_pressed(KeyCode::KeyA) {
            transform.translation -= right * controller.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyD) {
            transform.translation += right * controller.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyW) {
            transform.translation += forward * controller.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::KeyS) {
            transform.translation -= forward * controller.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::Space) {
            transform.translation += up * controller.speed * delta;
        }
        if keyboard.is_pressed(KeyCode::ShiftLeft) {
            transform.translation -= up * controller.speed * delta;
        }
        camera.projection = controller.projection();
    }
}

fn set_cam_projections(game: &mut Game, _ctx: RunContext) {
    let mut world       = game.get::<&mut World>();
    let window          = game.get::<&Window>();
    let win_size = window.size();

    for (_, (mut camera, controller)) in world.query_mut::<(&mut Camera, &mut CameraController)>() {
        camera.projection = controller.projection();
        match controller.scaling_mode {
            ScalingMode::Stretch => {},
            ScalingMode::ScaleSmallest => scale_smallest_viewport(
                win_size,
                controller.aspect_ratio(),
                &mut camera,
            ),
            ScalingMode::ScaleLargest => {},
        };
    }
}

fn scale_smallest_viewport(win_size: Vec2, aspect_ratio: f32, camera: &mut Camera) {
    let cam_w = aspect_ratio;
    let cam_h = 1.0;
    let scale_x = win_size.x / cam_w;
    let scale_y = win_size.y / cam_h;
    let proj_scale = if scale_x > scale_y {
        let des_size = Vec2::new(cam_w * scale_x, cam_h * scale_x);
        des_size / win_size
    }
    else {
        let des_size = Vec2::new(cam_w * scale_y, cam_h * scale_y);
        des_size / win_size
    };
    camera.projection *= Mat4::from_scale(Vec3::new(proj_scale.x, proj_scale.y, 1.0));
}

/// Camera projection component.
#[derive(Copy, Clone, Default, Debug)]
pub struct Camera {
    pub projection: Mat4,
    pub viewport: Option<Rect>,
}

pub struct CameraController {
    pub speed: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub sensitivity: f32,
    pub scroll_sensitivity: f32,
    pub orthographic: OrthographicProjector,
    pub perspective: PerspectiveProjector,
    pub t: f32,
    pub scaling_mode: ScalingMode,
    pub flycam_mode: FlycamMode,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            speed: 2.0,
            pitch: 0.0,
            yaw: 0.0,
            sensitivity: 1.0,
            scroll_sensitivity: 1.0,
            orthographic: OrthographicProjector::default(),
            perspective: PerspectiveProjector::default(),
            t: 1.0,
            scaling_mode: ScalingMode::default(),
            flycam_mode: FlycamMode::default(),
        }
    }
}

impl CameraController {

    pub fn rotation(&self) -> Quat {
        Quat::IDENTITY * Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch)
    }

    pub fn axes(&self) -> (Vec3, Vec3, Vec3) {
        let rotation = self.rotation();
        let x = rotation * Vec3::X;
        let x = Vec3::new(x.x, 0.0, x.z).normalize();
        let z = rotation * Vec3::NEG_Z;
        let z = Vec3::new(z.x, 0.0, z.z).normalize();
        (x, Vec3::Y, z)
    }

    /// Interpolated orthographic and perspective projections.
    /// t of 0.0 = orthographic
    /// t of 1.0 = perspective
    pub fn projection(&self) -> Mat4 {
        let ortho_proj = self.orthographic.compute_projection();
        let persp_proj = self.perspective.compute_projection();
        lerp_matrices(ortho_proj, persp_proj, self.t)
    }

    pub fn aspect_ratio(&self) -> f32 {
        let orth_width = self.orthographic.right - self.orthographic.left;
        let orth_height = self.orthographic.top - self.orthographic.bottom;
        let orth_asp = orth_width / orth_height;
        let persp_asp = self.perspective.aspect_ratio;
        orth_asp + (persp_asp - orth_asp) * self.t
    }
}

/**
 * Toggle for flycam mode
 */
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum FlycamMode {
    #[default]
    Off,
    Enabled,
    Disabled,
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub enum ScalingMode {
    /// Stretches image. Does not preserve aspect ratio.
    Stretch,
    /// Scales image. Preserves aspect ratio. Black bars.
    ScaleSmallest,
    /// Scales image. Preserves aspect ratio. Cuts off parts of image.
    #[default]
    ScaleLargest,
}

#[derive(Clone, Debug)]
pub struct OrthographicProjector {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicProjector {
    pub fn compute_projection(&self) -> Mat4 {
        Mat4::orthographic_rh(self.left, self.right, self.bottom, self.top, self.near, self.far)
    }
}

impl Default for OrthographicProjector {
    fn default() -> Self {
        Self {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PerspectiveProjector {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl PerspectiveProjector {
    pub fn compute_projection(&self) -> Mat4 {
        let fov = self.fov * PI / 180.0;
        Mat4::perspective_rh(fov, self.aspect_ratio, self.near, self.far)
    }
}

impl Default for PerspectiveProjector {
    fn default() -> Self {
         Self {
            fov: 80.0,
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 10000.0,
        }
    }
}