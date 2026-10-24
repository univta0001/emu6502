use std::{
    error::Error,
    mem::{offset_of, size_of},
};

use imgui::{DrawCmdParams, DrawIdx, DrawVert, TextureId, internal::RawWrapper};
use sdl3::{gpu::*, rect::Rect, video::Window};

use crate::utils::{create_buffer_with_data, create_texture};

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
    textures: Vec<(Texture<'static>, Sampler)>,
}

impl Renderer {
    /// Creates a new ImGui SDL3 renderer.
    ///
    /// This function builds a graphics pipeline from SPIR-V vertex/fragment shaders,
    /// configures the vertex input state to match `DrawVert`, and uploads the ImGui font atlas.
    pub fn new(device: &Device, window: &Window, imgui_context: &mut imgui::Context) -> Result<Self, Box<dyn Error>> {
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
            .build()?;

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
            .build()?;

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
            .build()?;

        // Upload the ImGui font texture to the GPU
        let font_texture = create_imgui_font_texture(device, imgui_context)?;
        // Create a texture sampler and bind font texture
        let sampler = device.create_sampler(
            SamplerCreateInfo::new()
                .with_min_filter(Filter::Linear)
                .with_mag_filter(Filter::Linear)
                .with_mipmap_mode(SamplerMipmapMode::Linear)
                .with_address_mode_u(SamplerAddressMode::ClampToEdge)
                .with_address_mode_v(SamplerAddressMode::ClampToEdge)
                .with_address_mode_w(SamplerAddressMode::ClampToEdge),
        )?;

        Ok(Self {
            pipeline,
            textures: vec![(font_texture, sampler)],
        })
    }

    pub fn push_texture(&mut self, texture: Texture<'static>, sampler: Sampler) -> TextureId {
        let id = TextureId::from(self.textures.len());
        self.textures.push((texture, sampler));
        id
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
        imgui_context: &mut imgui::Context,
    ) -> Result<(), Box<dyn Error>> {
        let io = imgui_context.io();
        let [width, height] = io.display_size;
        let [scale_w, scale_h] = io.display_framebuffer_scale;

        let fb_width = width * scale_w;
        let fb_height = height * scale_h;

        let draw_data = imgui_context.render();

        // Skip rendering if there's nothing to draw
        if width == 0.0 || height == 0.0 || draw_data.total_vtx_count == 0 || draw_data.total_idx_count == 0 {
            return Ok(());
        }

        let render_pass = device.begin_render_pass(command_buffer, color_targets, None)?;
        render_pass.bind_graphics_pipeline(&self.pipeline);

        // Flatten all draw data into a single vertex/index buffer
        let mut vtx_data = Vec::with_capacity(draw_data.total_vtx_count as usize);
        let mut idx_data = Vec::with_capacity(draw_data.total_idx_count as usize);
        for draw_list in draw_data.draw_lists() {
            vtx_data.extend_from_slice(draw_list.vtx_buffer());
            idx_data.extend_from_slice(draw_list.idx_buffer());
        }

        // Create a buffer for transfer and copy data
        let copy_commands = device.acquire_command_buffer()?;
        let transfer_buffer = device
            .create_transfer_buffer()
            .with_size((vtx_data.len().max(idx_data.len()) * std::mem::size_of::<DrawVert>()) as u32)
            .with_usage(sdl3::gpu::TransferBufferUsage::UPLOAD)
            .build()?;

        let copy_pass = device.begin_copy_pass(&copy_commands)?;

        let vertex_buffer = create_buffer_with_data(
            device,
            &transfer_buffer,
            &copy_pass,
            sdl3::gpu::BufferUsageFlags::VERTEX,
            &vtx_data,
        )?;

        let index_buffer = create_buffer_with_data(
            device,
            &transfer_buffer,
            &copy_pass,
            sdl3::gpu::BufferUsageFlags::INDEX,
            &idx_data,
        )?;

        device.end_copy_pass(copy_pass);
        copy_commands.submit()?;

        // Bind vertex and index buffers
        render_pass.bind_vertex_buffers(0, &[BufferBinding::new().with_buffer(&vertex_buffer).with_offset(0)]);
        render_pass.bind_index_buffer(
            &BufferBinding::new().with_buffer(&index_buffer).with_offset(0),
            if size_of::<DrawIdx>() == 2 {
                IndexElementSize::_16BIT
            } else {
                IndexElementSize::_32BIT
            },
        );

        // Set viewport and projection matrix
        device.set_viewport(&render_pass, Viewport::new(0.0, 0.0, fb_width, fb_height, 0.0, 1.0));

        // Push orthographic projection matrix
        let matrix = [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, 2.0 / -height, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];
        command_buffer.push_vertex_uniform_data(0, &matrix);

        // Render each draw command
        let mut voffset = 0;
        let mut ioffset = 0;

        for draw_list in draw_data.draw_lists() {
            for draw_cmd in draw_list.commands() {
                match draw_cmd {
                    imgui::DrawCmd::Elements {
                        count,
                        cmd_params:
                            DrawCmdParams {
                                clip_rect: [x, y, w, h],
                                idx_offset,
                                vtx_offset,
                                texture_id,
                            },
                    } => {
                        // Calculate scissor rectangle
                        let scissor_x = (x * scale_w) as i32;
                        let scissor_y = (y * scale_h) as i32;
                        let scissor_w = ((w - x) * scale_w).max(0.0) as u32;
                        let scissor_h = ((h - y) * scale_h).max(0.0) as u32;

                        // Skip if scissor is invalid
                        if scissor_w > 0 && scissor_h > 0 {
                            render_pass.set_scissor(Rect::new(scissor_x, scissor_y, scissor_w, scissor_h));
                        } else {
                            continue;
                        }

                        let (t, s) = &self.textures[texture_id.id()];
                        let binding = TextureSamplerBinding::new().with_texture(t).with_sampler(s);
                        render_pass.bind_fragment_samplers(0, &[binding]);

                        // Draw the elements
                        render_pass.draw_indexed_primitives(
                            count as u32,
                            1,
                            (idx_offset + ioffset) as u32,
                            (vtx_offset + voffset) as i32,
                            0,
                        );
                    }

                    imgui::DrawCmd::RawCallback { callback, raw_cmd } => unsafe {
                        callback(draw_list.raw(), raw_cmd);
                    },

                    _ => {}
                }
            }

            ioffset += draw_list.idx_buffer().len();
            voffset += draw_list.vtx_buffer().len();
        }

        device.end_render_pass(render_pass);

        // Clear all textures except the font, which is always 0.
        self.textures.drain(1..);

        Ok(())
    }
}

/// Uploads the ImGui font atlas to the GPU and returns the resulting texture.
fn create_imgui_font_texture(
    device: &Device,
    imgui_context: &mut imgui::Context,
) -> Result<Texture<'static>, Box<dyn Error>> {
    let font_atlas = imgui_context.fonts().build_rgba32_texture();

    let copy_commands = device.acquire_command_buffer()?;
    let copy_pass = device.begin_copy_pass(&copy_commands)?;

    let font_texture = create_texture(device, &copy_pass, font_atlas.data, font_atlas.width, font_atlas.height)?;

    device.end_copy_pass(copy_pass);
    copy_commands.submit()?;

    // Assign the font texture ID (hardcoded to 0)
    imgui_context.fonts().tex_id = imgui::TextureId::from(0);

    Ok(font_texture)
}
