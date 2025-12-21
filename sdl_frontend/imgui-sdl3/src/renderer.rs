use std::{
    error::Error,
    mem::{offset_of, size_of},
};

use crate::utils::create_texture;
use imgui::{Context, DrawCmdParams, DrawIdx, DrawVert, TextureId, Textures, internal::RawWrapper};
use sdl3::{gpu::*, rect::Rect, video::Window};
use std::fmt;

// Custom error type for better error handling
#[derive(Debug)]
pub enum RendererError {
    ShaderCreation(String),
    PipelineCreation(String),
    TextureCreation(String),
    BufferCreation(String),
    RenderCommand(String),
}

impl fmt::Display for RendererError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RendererError::ShaderCreation(msg) => write!(f, "Shader creation failed: {}", msg),
            RendererError::PipelineCreation(msg) => write!(f, "Pipeline creation failed: {}", msg),
            RendererError::TextureCreation(msg) => write!(f, "Texture creation failed: {}", msg),
            RendererError::BufferCreation(msg) => write!(f, "Buffer creation failed: {}", msg),
            RendererError::RenderCommand(msg) => write!(f, "Render command failed: {}", msg),
        }
    }
}

impl Error for RendererError {}

// Size of the ring buffers. Adjust based on expected scene complexity.
const RING_BUFFER_SIZE: u32 = 512 * 1024; // 500 KB
//
//
/// Renderer backend for imgui using SDL3 GPU.
///
/// This renderer performs the following tasks:
///
/// * Initializes a pipeline with blending suitable for ImGui
/// * Uploads the ImGui font atlas as a GPU texture
/// * Creates GPU buffers every frame for ImGui vertex/index data
/// * Issues draw calls using ImGui's draw list
pub struct Renderer {
    pipeline: GraphicsPipeline,
    sampler: Sampler,
    textures: Textures<Texture<'static>>,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    vtx_data: Vec<DrawVert>,
    idx_data: Vec<DrawIdx>,
}

impl Renderer {
    /// Creates a new ImGui SDL3 renderer.
    ///
    /// This function builds a graphics pipeline from SPIR-V vertex/fragment shaders,
    /// configures the vertex input state to match `DrawVert`, and uploads the ImGui font atlas.
    pub fn new(device: &Device, window: &Window, imgui_context: &mut imgui::Context) -> Result<Self, Box<dyn Error>> {
        let sampler = device.create_sampler(
            SamplerCreateInfo::new()
                .with_min_filter(Filter::Linear)
                .with_mag_filter(Filter::Linear)
                .with_mipmap_mode(SamplerMipmapMode::Linear)
                .with_address_mode_u(SamplerAddressMode::ClampToEdge)
                .with_address_mode_v(SamplerAddressMode::ClampToEdge)
                .with_address_mode_w(SamplerAddressMode::ClampToEdge),
        )?;

        // Load and configure vertex shader
        let vert = device
            .create_shader()
            .with_code(
                ShaderFormat::SPIRV,
                include_bytes!(concat!(env!("OUT_DIR"), "/imgui.vert.spv")),
                ShaderStage::Vertex,
            )
            .with_uniform_buffers(1)
            .with_entrypoint(c"main")
            .build()
            .map_err(|e| RendererError::ShaderCreation(format!("Vertex shader: {:?}", e)))?;

        // Load and configure fragment shader
        let frag = device
            .create_shader()
            .with_code(
                ShaderFormat::SPIRV,
                include_bytes!(concat!(env!("OUT_DIR"), "/imgui.frag.spv")),
                ShaderStage::Fragment,
            )
            .with_samplers(1)
            .with_entrypoint(c"main")
            .build()
            .map_err(|e| RendererError::ShaderCreation(format!("Fragment shader: {:?}", e)))?;

        let format = device.get_swapchain_texture_format(window);

        // Build the graphics pipeline
        let pipeline = device
            .create_graphics_pipeline()
            .with_vertex_shader(&vert)
            .with_vertex_input_state(
                VertexInputState::new()
                    .with_vertex_buffer_descriptions(&[VertexBufferDescription::new()
                        .with_slot(0)
                        .with_pitch(size_of::<DrawVert>() as u32)
                        .with_input_rate(VertexInputRate::Vertex)
                        .with_instance_step_rate(0)])
                    .with_vertex_attributes(&[
                        // Position
                        VertexAttribute::new()
                            .with_format(VertexElementFormat::Float2)
                            .with_location(0)
                            .with_buffer_slot(0)
                            .with_offset(offset_of!(DrawVert, pos) as u32),
                        // UV
                        VertexAttribute::new()
                            .with_format(VertexElementFormat::Float2)
                            .with_location(1)
                            .with_buffer_slot(0)
                            .with_offset(offset_of!(DrawVert, uv) as u32),
                        // Color
                        VertexAttribute::new()
                            .with_format(VertexElementFormat::Ubyte4Norm)
                            .with_location(2)
                            .with_buffer_slot(0)
                            .with_offset(offset_of!(DrawVert, col) as u32),
                    ]),
            )
            .with_rasterizer_state(
                RasterizerState::new()
                    .with_fill_mode(FillMode::Fill)
                    .with_front_face(FrontFace::Clockwise), // Disable culling for UI geometry
            )
            .with_fragment_shader(&frag)
            .with_primitive_type(PrimitiveType::TriangleList)
            .with_target_info(
                GraphicsPipelineTargetInfo::new().with_color_target_descriptions(&[ColorTargetDescription::new()
                    .with_format(format)
                    .with_blend_state(
                        ColorTargetBlendState::new()
                            .with_color_blend_op(BlendOp::Add)
                            .with_src_color_blendfactor(BlendFactor::SrcAlpha)
                            .with_dst_color_blendfactor(BlendFactor::OneMinusSrcAlpha)
                            .with_alpha_blend_op(BlendOp::Add)
                            .with_src_alpha_blendfactor(BlendFactor::One)
                            .with_dst_alpha_blendfactor(BlendFactor::OneMinusSrcAlpha)
                            .with_enable_blend(true),
                    )]),
            )
            .build()
            .map_err(|e| RendererError::PipelineCreation(format!("{:?}", e)))?;

        // Create persistent ring buffers
        let vertex_buffer = device
            .create_buffer()
            .with_size(RING_BUFFER_SIZE)
            .with_usage(BufferUsageFlags::VERTEX)
            .build()
            .map_err(|e| RendererError::BufferCreation(format!("Vertex ring buffer: {:?}", e)))?;

        let index_buffer = device
            .create_buffer()
            .with_size(RING_BUFFER_SIZE)
            .with_usage(BufferUsageFlags::INDEX)
            .build()
            .map_err(|e| RendererError::BufferCreation(format!("Index ring buffer: {:?}", e)))?;

        // Upload the ImGui font texture to the GPU
        let font_texture = create_imgui_font_texture(device, imgui_context)?;
        let mut textures = Textures::default();
        textures.insert(font_texture);
        Ok(Self {
            pipeline,
            sampler,
            textures,
            vertex_buffer,
            index_buffer,
            vtx_data: Vec::with_capacity(1024),
            idx_data: Vec::with_capacity(1024),
        })
    }

    /// Renders the current ImGui draw data into the window.
    ///
    /// This function:
    /// * Builds and submits GPU buffers from draw data
    /// * Sets an orthographic projection matrix
    /// * Issues indexed draw calls
    pub fn render(
        &mut self,
        device: &Device,
        command_buffer: &mut CommandBuffer,
        color_targets: &[ColorTargetInfo],
        imgui_context: &mut Context,
    ) -> Result<(), Box<dyn Error>> {
        let io = imgui_context.io();
        let [width, height] = io.display_size;

        let draw_data = imgui_context.render();

        // Skip rendering if there's nothing to draw
        if width == 0.0 || height == 0.0 || draw_data.total_vtx_count == 0 || draw_data.total_idx_count == 0 {
            return Ok(());
        }

        // 1. Prepare data on CPU
        self.prepare_draw_data(draw_data);

        // 2. Upload data to GPU ring buffers
        self.upload_data_to_gpu(device, command_buffer)?;

        // 3. Render the draw calls
        let render_pass = device.begin_render_pass(command_buffer, color_targets, None)?;
        self.execute_draw_pass(device, &render_pass, command_buffer, draw_data)?;

        device.end_render_pass(render_pass);

        Ok(())
    }

    fn prepare_draw_data(&mut self, draw_data: &imgui::DrawData) {
        self.vtx_data.clear();
        self.idx_data.clear();

        // Reserve capacity to avoid reallocations
        self.vtx_data.reserve(draw_data.total_vtx_count as usize);
        self.idx_data.reserve(draw_data.total_idx_count as usize);

        for draw_list in draw_data.draw_lists() {
            self.vtx_data.extend_from_slice(draw_list.vtx_buffer());
            self.idx_data.extend_from_slice(draw_list.idx_buffer());
        }
    }

    /// Uploads the prepared CPU data to the GPU ring buffers.
    fn upload_data_to_gpu(
        &mut self,
        device: &Device,
        command_buffer: &mut CommandBuffer,
    ) -> Result<(), Box<dyn Error>> {
        let vtx_bytes = (self.vtx_data.len() * size_of::<DrawVert>()) as u32;
        let idx_bytes = (self.idx_data.len() * size_of::<DrawIdx>()) as u32;

        // Use a temporary transfer buffer for the upload
        let transfer_buffer = device
            .create_transfer_buffer()
            .with_size(vtx_bytes.max(idx_bytes) as u32)
            .with_usage(TransferBufferUsage::UPLOAD)
            .build()?;

        // Begin a copy pass to upload data
        let copy_pass = device.begin_copy_pass(command_buffer)?;

        let mut map = transfer_buffer.map::<DrawVert>(device, true);
        let mem = map.mem_mut();
        mem[..self.vtx_data.len()].copy_from_slice(&self.vtx_data);
        map.unmap();

        // Upload vertex data
        copy_pass.upload_to_gpu_buffer(
            TransferBufferLocation::new()
                .with_transfer_buffer(&transfer_buffer)
                .with_offset(0),
            BufferRegion::new()
                .with_buffer(&self.vertex_buffer)
                .with_offset(0)
                .with_size(vtx_bytes),
            false,
        );

        let mut map = transfer_buffer.map::<DrawIdx>(device, true);
        let mem = map.mem_mut();
        mem[..self.idx_data.len()].copy_from_slice(&self.idx_data);
        map.unmap();

        // Upload index data
        copy_pass.upload_to_gpu_buffer(
            TransferBufferLocation::new()
                .with_transfer_buffer(&transfer_buffer)
                .with_offset(0),
            BufferRegion::new()
                .with_buffer(&self.index_buffer)
                .with_offset(0)
                .with_size(idx_bytes),
            false,
        );

        device.end_copy_pass(copy_pass);

        Ok(())
    }

    /// Executes the main render pass, drawing all commands.
    fn execute_draw_pass(
        &mut self,
        device: &Device,
        render_pass: &RenderPass,
        command_buffer: &mut CommandBuffer,
        draw_data: &imgui::DrawData,
    ) -> Result<(), Box<dyn Error>> {
        let width = draw_data.display_size[0];
        let height = draw_data.display_size[1];
        let scale_w = draw_data.framebuffer_scale[0];
        let scale_h = draw_data.framebuffer_scale[1];

        render_pass.bind_graphics_pipeline(&self.pipeline);

        render_pass.bind_vertex_buffers(
            0,
            &[BufferBinding::new().with_buffer(&self.vertex_buffer).with_offset(0)],
        );
        render_pass.bind_index_buffer(
            &BufferBinding::new().with_buffer(&self.index_buffer).with_offset(0),
            if size_of::<DrawIdx>() == 2 {
                IndexElementSize::_16BIT
            } else {
                IndexElementSize::_32BIT
            },
        );

        // Set viewport
        device.set_viewport(
            render_pass,
            Viewport::new(0.0, 0.0, width * scale_w, height * scale_h, 0.0, 1.0),
        );

        let offset_x = draw_data.display_pos[0] / width;
        let offset_y = draw_data.display_pos[1] / height;

        // Push orthographic projection matrix
        let matrix = [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, 2.0 / -height, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [-1.0 - offset_x * 2.0, 1.0 + offset_y * 2.0, 0.0, 1.0],
        ];
        command_buffer.push_vertex_uniform_data(0, &matrix);

        // Render each draw command
        let mut v_offset = 0;
        let mut i_offset = 0;
        let mut current_texture_id = usize::MAX; // Use an invalid ID to force first bind

        for draw_list in draw_data.draw_lists() {
            for draw_cmd in draw_list.commands() {
                match draw_cmd {
                    imgui::DrawCmd::Elements {
                        count,
                        cmd_params:
                            DrawCmdParams {
                                texture_id,
                                clip_rect: [x, y, w, h],
                                idx_offset,
                                vtx_offset,
                                ..
                            },
                    } => {
                        // Calculate scissor rectangle
                        let scissor_x = (x * scale_w) as i32;
                        let scissor_y = (y * scale_h) as i32;
                        let scissor_w = ((w - x) * scale_w).max(0.0) as u32;
                        let scissor_h = ((h - y) * scale_h).max(0.0) as u32;

                        // Skip if scissor is invalid
                        if scissor_w == 0 || scissor_h == 0 {
                            continue;
                        }
                        render_pass.set_scissor(Rect::new(scissor_x, scissor_y, scissor_w, scissor_h));

                        // Bind texture if it has changed
                        if texture_id.id() != current_texture_id
                            && let Some(texture) = self.textures.get(texture_id)
                        {
                            let sampler_binding = TextureSamplerBinding::new()
                                .with_texture(texture)
                                .with_sampler(&self.sampler);
                            render_pass.bind_fragment_samplers(0, &[sampler_binding]);
                            current_texture_id = texture_id.id();
                        }

                        // Draw the elements
                        render_pass.draw_indexed_primitives(
                            count as u32,
                            1,
                            (idx_offset + i_offset) as u32,
                            (vtx_offset + v_offset) as i32,
                            0,
                        );
                    }
                    imgui::DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                        callback(draw_list.raw(), raw_cmd);
                    },
                    _ => {}
                }
            }
            i_offset += draw_list.idx_buffer().len();
            v_offset += draw_list.vtx_buffer().len();
        }

        Ok(())
    }

    pub fn insert_texture(&mut self, texture: Texture<'static>) -> TextureId {
        self.textures.insert(texture)
    }

    pub fn replace_texture(&mut self, id: TextureId, texture: Texture<'static>) -> Option<Texture<'static>> {
        self.textures.replace(id, texture)
    }

    pub fn remove_texture(&mut self, id: TextureId) -> Option<Texture<'static>> {
        self.textures.remove(id)
    }

    pub fn get_texture(&self, id: TextureId) -> Option<&Texture<'static>> {
        self.textures.get(id)
    }
}

/// Uploads the ImGui font atlas to the GPU and returns the resulting texture.
fn create_imgui_font_texture(device: &Device, imgui_context: &mut Context) -> Result<Texture<'static>, Box<dyn Error>> {
    let font_atlas = imgui_context.fonts().build_rgba32_texture();

    let copy_commands = device.acquire_command_buffer()?;
    let copy_pass = device.begin_copy_pass(&copy_commands)?;

    let font_texture = create_texture(device, &copy_pass, font_atlas.data, font_atlas.width, font_atlas.height)
        .map_err(|e| RendererError::TextureCreation(format!("Font texture: {:?}", e)))?;

    device.end_copy_pass(copy_pass);
    copy_commands
        .submit()
        .map_err(|e| RendererError::RenderCommand(format!("Font texture upload: {:?}", e)))?;

    // Assign the font texture ID (hardcoded to 0)
    imgui_context.fonts().tex_id = imgui::TextureId::from(0);

    Ok(font_texture)
}
