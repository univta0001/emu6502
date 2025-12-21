//! # Getting started
//!
//! ```rust,no_run
//! use imgui_sdl3::ImGuiSdl3;
//! use sdl3::{event::Event, gpu::*, pixels::Color};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // initialize SDL and its video subsystem
//!     let mut sdl = sdl3::init().unwrap();
//!     let video_subsystem = sdl.video().unwrap();
//!
//!     // create a new window
//!     let window = video_subsystem
//!         .window("Hello imgui-rs!", 1280, 720)
//!         .position_centered()
//!         .resizable()
//!         .build()
//!         .unwrap();
//!
//!     let device = Device::new(ShaderFormat::SPIRV, true)
//!         .unwrap()
//!         .with_window(&window)
//!         .unwrap();
//!
//!     // create platform and renderer
//!     let mut imgui = ImGuiSdl3::new(&device, &window, |ctx| {
//!         // disable creation of files on disc
//!         ctx.set_ini_filename(None);
//!         ctx.set_log_filename(None);
//!
//!         // setup platform and renderer, and fonts to imgui
//!         ctx.fonts()
//!             .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);
//!     });
//!
//!     // start main loop
//!     let mut event_pump = sdl.event_pump().unwrap();
//!
//!     'main: loop {
//!         for event in event_pump.poll_iter() {
//!             // pass all events to imgui platform
//!             imgui.handle_event(&event);
//!
//!             if let Event::Quit { .. } = event {
//!                 break 'main;
//!             }
//!         }
//!
//!         let mut command_buffer = device.acquire_command_buffer()?;
//!
//!         if let Ok(swapchain) = command_buffer.wait_and_acquire_swapchain_texture(&window) {
//!             let color_targets = [ColorTargetInfo::default()
//!                 .with_texture(&swapchain)
//!                 .with_load_op(LoadOp::CLEAR)
//!                 .with_store_op(StoreOp::STORE)
//!                 .with_clear_color(Color::RGB(128, 128, 128))];
//!
//!             imgui.render(
//!                 &mut sdl,
//!                 &device,
//!                 &window,
//!                 &event_pump,
//!                 &mut command_buffer,
//!                 &color_targets,
//!                 |ui| {
//!                     // create imgui UI here
//!                     ui.show_demo_window(&mut true);
//!                 },
//!             );
//!
//!             command_buffer.submit()?;
//!         } else {
//!             println!("Swapchain unavailable, cancel work");
//!             command_buffer.cancel();
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

#![crate_name = "imgui_sdl3"]
#![crate_type = "lib"]

pub mod platform;
pub mod renderer;
pub mod utils;
use platform::Platform;
use renderer::Renderer;
use sdl3::gpu::*;

/// Main integration point for using Dear ImGui with SDL3 + GPU rendering
pub struct ImGuiSdl3 {
    imgui_context: imgui::Context, // Dear ImGui context (state, configuration, fonts, etc.)
    platform: Platform,            // Handles SDL3 platform event integration
    renderer: Renderer,            // Handles GPU rendering of ImGui draw data
}

impl ImGuiSdl3 {
    /// Create a new ImGuiSdl3 instance
    ///
    /// - `device`: GPU device handle from SDL3
    /// - `window`: SDL3 window reference
    /// - `ctx_configure`: Closure to configure the ImGui context (fonts, styles, etc.)
    pub fn new<T>(device: &sdl3::gpu::Device, window: &sdl3::video::Window, ctx_configure: T) -> Self
    where
        T: Fn(&mut imgui::Context), // Allows custom configuration of the ImGui context
    {
        // Create a fresh Dear ImGui context
        let mut imgui_context = imgui::Context::create();

        // Apply user-provided configuration to the context
        ctx_configure(&mut imgui_context);

        // Set up SDL3 platform integration (input handling, DPI scaling, etc.)
        let platform = Platform::new(&mut imgui_context);

        // Set up the GPU renderer for drawing ImGui's UI
        let renderer = Renderer::new(device, window, &mut imgui_context).unwrap();

        Self {
            imgui_context,
            platform,
            renderer,
        }
    }

    /// Pass SDL3 events to ImGui so it can handle inputs (mouse, keyboard, etc.)
    pub fn handle_event(&mut self, event: &sdl3::event::Event) {
        self.platform.handle_event(&mut self.imgui_context, event);
    }

    /// Render an ImGui frame
    ///
    /// - `sdl_context`: SDL3 main context
    /// - `device`: GPU device handle
    /// - `window`: SDL3 window reference
    /// - `event_pump`: SDL3 event pump for polling events
    /// - `command_buffer`: GPU command buffer for recording draw commands
    /// - `color_targets`: Color target attachments for rendering
    /// - `draw_callback`: Closure to build the UI each frame
    #[allow(clippy::too_many_arguments)]
    pub fn render<T>(
        &mut self,
        sdl_context: &mut sdl3::Sdl,
        device: &sdl3::gpu::Device,
        window: &sdl3::video::Window,
        event_pump: &sdl3::EventPump,
        command_buffer: &mut CommandBuffer,
        color_targets: &[ColorTargetInfo],
        mut draw_callback: T,
    ) where
        T: FnMut(&mut imgui::Ui), // Function that takes a mutable reference to the UI builder
    {
        // Prepare ImGui for a new frame (update input state, time step, etc.)
        self.platform
            .prepare_frame(sdl_context, &mut self.imgui_context, window, event_pump);

        // Start a new ImGui frame and get the UI object
        let ui = self.imgui_context.new_frame();

        // Call the user-provided draw function to build the UI
        draw_callback(ui);

        // Render the ImGui draw data to the GPU
        self.renderer
            .render(device, command_buffer, color_targets, &mut self.imgui_context)
            .unwrap();
    }

    pub fn insert_texture(&mut self, texture: Texture<'static>) -> imgui::TextureId {
        self.renderer.insert_texture(texture)
    }

    pub fn replace_texture(&mut self, id: imgui::TextureId, texture: Texture<'static>) -> Option<Texture<'static>> {
        self.renderer.replace_texture(id, texture)
    }

    pub fn remove_texture(&mut self, id: imgui::TextureId) -> Option<Texture<'static>> {
        self.renderer.remove_texture(id)
    }

    pub fn get_texture(&self, id: imgui::TextureId) -> Option<&Texture<'static>> {
        self.renderer.get_texture(id)
    }
}
