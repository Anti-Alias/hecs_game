use std::sync::Arc;
use winit::window::Window;
use wgpu::*;

/**
 * Stores WGPU primitives needed to do any and all graphics operations.
 */
pub struct GraphicsState {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    surface: Surface,
    surface_config: SurfaceConfiguration,
}

impl GraphicsState {

    pub fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(InstanceDescriptor::default());
        let surface = unsafe {
            instance.create_surface(window).expect("Failed to create surface")
        };
        let adapter = instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        });
        let adapter = pollster::block_on(adapter).expect("Compatible adapter not found");
        let device_queue = adapter.request_device(&DeviceDescriptor::default(), None);
        let (device, queue) = pollster::block_on(device_queue).expect("Failed to request device");
        let window_size = window.inner_size();
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: window_size.width,
            height: window_size.height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);
        Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface,
            surface_config,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    /**
     * Current texture view to render on.
    */
    pub fn surface(&self) -> &Surface {
        &self.surface
    }

    pub fn surface_config(&self) -> &SurfaceConfiguration {
        &self.surface_config
    }
}