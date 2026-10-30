use std::{
    collections::HashMap,
    error::Error,
    mem::{offset_of, size_of},
};

use imgui::{DrawCmdParams, DrawIdx, DrawVert, TextureId, internal::RawWrapper, sys};
use sdl3::{gpu::*, rect::Rect, video::Window};

use crate::utils::{create_buffer_with_data, create_texture, upload_to_gpu_texture};

/// Renderer backend for imgui using SDL3 GPU.
///
/// This renderer performs the following tasks:
///
/// * Initializes a pipeline with blending suitable for ImGui
/// * Creates, updates and destroys GPU textures on imgui's request
///   (Dear ImGui 1.92+ `ImGuiBackendFlags_RendererHasTextures` protocol,
///   which covers the font atlas)
/// * Creates GPU buffers every frame for ImGui vertex/index data
/// * Issues draw calls using ImGui's draw list
pub struct Renderer {
    pipeline: GraphicsPipeline,
    /// All live GPU textures, keyed by the `TextureId` referenced in draw data.
    /// Holds both textures created at imgui's request (the font atlas) and the
    /// per-frame textures pushed by the application.
    textures: HashMap<TextureId, (Texture<'static>, Sampler)>,
    /// Ids of textures pushed by the application this frame; they are dropped
    /// at the end of the frame.
    frame_texture_ids: Vec<TextureId>,
    /// Sampler shared by every texture created on imgui's request.
    imgui_sampler: Sampler,
    /// Monotonic counter handing out `TextureId`s so ids of imgui-created and
    /// application-pushed textures never collide.
    next_texture_id: usize,
}

impl Renderer {
    /// Creates a new ImGui SDL3 renderer.
    ///
    /// This function builds a graphics pipeline from SPIR-V vertex/fragment shaders,
    /// configures the vertex input state to match `DrawVert`, and advertises texture
    /// management to ImGui (which then owns the font atlas texture).
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

        // Advertise support for the Dear ImGui 1.92+ texture-request protocol.
        // This must be set before the first `NewFrame()`: from then on ImGui owns
        // building and updating the font atlas texture, and it signals us through
        // `platform_io().textures` (see `Renderer::update_textures`).
        imgui_context.io_mut().backend_flags |= imgui::BackendFlags::RENDERER_HAS_TEXTURES;

        // Sampler shared by every texture created at ImGui's request (the font atlas).
        let imgui_sampler = device.create_sampler(
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
            textures: HashMap::new(),
            frame_texture_ids: Vec::new(),
            imgui_sampler,
            // `TextureId` 0 is `ImTextureID_Invalid`; hand out ids from 1 upward.
            next_texture_id: 1,
        })
    }

    /// Registers a texture pushed by the application and returns its id.
    ///
    /// The texture lives until the end of the current frame; the application is
    /// expected to push a fresh one each frame (see `Renderer::render`, which
    /// drops it after the draw calls that reference it have been issued).
    pub fn push_texture(&mut self, texture: Texture<'static>, sampler: Sampler) -> TextureId {
        let id = TextureId::from(self.next_texture_id);
        self.next_texture_id += 1;
        self.textures.insert(id, (texture, sampler));
        self.frame_texture_ids.push(id);
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

        // Finalize the frame and produce draw data. This internally runs
        // EndFrame(), which is what populates ImGui's texture-request list
        // (platform_io().textures) for this frame.
        let draw_data = imgui_context.render();

        // Service any texture requests (font atlas create/update/destroy). Draw
        // commands resolve the font texture lazily via ImTextureRef_GetTexID, so
        // assigning the TexID here -- after render(), before we iterate the draw
        // commands -- is correct. This must run every frame, even when there is
        // nothing to draw, otherwise ImGui asserts on the following frame.
        self.update_textures(device)?;

        // Skip rendering if there's nothing to draw
        if width == 0.0 || height == 0.0 || draw_data.total_vtx_count == 0 || draw_data.total_idx_count == 0 {
            self.drop_frame_textures();
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

                        let (t, s) = self.textures.get(&texture_id).unwrap_or_else(|| {
                            panic!(
                                "draw command references unregistered texture {:?} (live: {:?})",
                                texture_id.id(),
                                self.textures.keys().map(|k| k.id()).collect::<Vec<_>>()
                            )
                        });
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

        // Drop the application-pushed textures; imgui-owned textures (the font
        // atlas) are not in `frame_texture_ids` and stay live.
        self.drop_frame_textures();

        Ok(())
    }

    /// Removes the textures the application pushed this frame. imgui-owned
    /// textures are unaffected.
    fn drop_frame_textures(&mut self) {
        for id in self.frame_texture_ids.drain(..) {
            self.textures.remove(&id);
        }
    }

    /// Services ImGui's texture requests (Dear ImGui 1.92+
    /// `ImGuiBackendFlags_RendererHasTextures` protocol).
    ///
    /// ImGui owns the font atlas texture and, during `new_frame()`, queues
    /// requests in `platform_io().textures`: create a texture, apply updates,
    /// or destroy one. We must fulfil each request and acknowledge it by setting
    /// the status back to `OK` (or `Destroyed`), otherwise the next `new_frame()`
    /// asserts. `Updates` are dirty sub-rects into the in-place `Pixels` buffer,
    /// so a full re-upload is always correct.
    fn update_textures(&mut self, device: &Device) -> Result<(), Box<dyn Error>> {
        // Access the texture request list through the sys layer: the safe
        // `Context::platform_io()` wrapper is only available with imgui's
        // `docking` feature, which this frontend does not enable.
        let textures = unsafe { &(*sys::igGetPlatformIO()).Textures };
        if textures.Size <= 0 {
            return Ok(());
        }
        let entries = unsafe { std::slice::from_raw_parts(textures.Data, textures.Size as usize) };

        // Only spin up a copy pass if there is something to upload this frame.
        let needs_upload = entries.iter().any(|&tex| {
            let status = unsafe { (*tex).Status };
            status == sys::ImTextureStatus_WantCreate || status == sys::ImTextureStatus_WantUpdates
        });

        let mut copy_commands = None;
        let mut copy_pass = None;
        if needs_upload {
            let cmd = device.acquire_command_buffer()?;
            copy_pass = Some(device.begin_copy_pass(&cmd)?);
            copy_commands = Some(cmd);
        }

        for &tex_ptr in entries {
            let tex = unsafe { &mut *tex_ptr };
            match tex.Status {
                s if s == sys::ImTextureStatus_WantCreate => {
                    let width = tex.Width as u32;
                    let height = tex.Height as u32;
                    let len = (width * height * tex.BytesPerPixel as u32) as usize;
                    let pixels = unsafe { std::slice::from_raw_parts(tex.Pixels, len) };

                    let texture = create_texture(
                        device,
                        copy_pass.as_ref().expect("copy pass open for WantCreate"),
                        pixels,
                        width,
                        height,
                    )?;

                    let id = TextureId::from(self.next_texture_id);
                    self.next_texture_id += 1;
                    self.textures.insert(id, (texture, self.imgui_sampler.clone()));

                    unsafe {
                        sys::ImTextureData_SetTexID(tex_ptr, id.id() as sys::ImTextureID);
                        sys::ImTextureData_SetStatus(tex_ptr, sys::ImTextureStatus_OK);
                    }
                }

                s if s == sys::ImTextureStatus_WantUpdates => {
                    let id = TextureId::from(tex.TexID as usize);
                    let width = tex.Width as u32;
                    let height = tex.Height as u32;
                    let len = (width * height * tex.BytesPerPixel as u32) as usize;
                    let pixels = unsafe { std::slice::from_raw_parts(tex.Pixels, len) };

                    if let Some((texture, _)) = self.textures.get(&id) {
                        upload_to_gpu_texture(
                            device,
                            copy_pass.as_ref().expect("copy pass open for WantUpdates"),
                            texture,
                            pixels,
                            width,
                            height,
                        )?;
                    }

                    unsafe { sys::ImTextureData_SetStatus(tex_ptr, sys::ImTextureStatus_OK) };
                }

                s if s == sys::ImTextureStatus_WantDestroy => {
                    let id = TextureId::from(tex.TexID as usize);
                    self.textures.remove(&id);
                    unsafe { sys::ImTextureData_SetStatus(tex_ptr, sys::ImTextureStatus_Destroyed) };
                }

                _ => {}
            }
        }

        if let Some(pass) = copy_pass.take() {
            device.end_copy_pass(pass);
        }
        if let Some(cmd) = copy_commands.take() {
            cmd.submit()?;
        }

        Ok(())
    }
}
