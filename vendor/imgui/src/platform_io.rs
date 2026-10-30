use std::ffi::{c_char, c_void};

use crate::internal::RawCast;

/// Holds the information needed to enable multiple viewports.
#[cfg(feature = "docking")]
#[repr(C)]
pub struct PlatformIo {
    pub(crate) get_clipboard_text_fn:
        Option<unsafe extern "C" fn(*mut sys::ImGuiContext) -> *const c_char>,
    pub(crate) set_clipboard_text_fn:
        Option<unsafe extern "C" fn(*mut sys::ImGuiContext, *const c_char)>,
    pub(crate) clipboard_user_data: *mut c_void,
    pub(crate) open_in_shell_fn:
        Option<unsafe extern "C" fn(ctx: *mut sys::ImGuiContext, path: *const c_char) -> bool>,
    pub(crate) open_in_shell_user_data: *mut c_void,
    pub(crate) set_ime_data_fn: Option<
        unsafe extern "C" fn(
            ctx: *mut sys::ImGuiContext,
            viewport: *mut sys::ImGuiViewport,
            data: *mut sys::ImGuiPlatformImeData,
        ),
    >,
    pub(crate) ime_user_data: *mut c_void,
    pub(crate) locale_decimal_point: sys::ImWchar,
    platform_session_date: i32,
    pub renderer_texture_max_width: i32,
    pub renderer_texture_max_height: i32,
    pub renderer_render_state: *mut c_void,
    draw_callback_reset_render_state: sys::ImDrawCallback,
    draw_callback_set_sampler_linear: sys::ImDrawCallback,
    draw_callback_set_sampler_nearest: sys::ImDrawCallback,
    pub platform_create_window: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport)>,
    pub platform_destroy_window: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport)>,
    pub platform_show_window: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport)>,
    pub platform_set_window_pos:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, sys::ImVec2_c)>,
    pub platform_get_window_pos: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport) -> sys::ImVec2_c>,
    pub platform_set_window_size:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, sys::ImVec2_c)>,
    pub platform_get_window_size:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport) -> sys::ImVec2_c>,
    pub platform_get_window_framebuffer_scale:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport) -> sys::ImVec2_c>,
    pub platform_set_window_focus: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport)>,
    pub platform_get_window_focus: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport) -> bool>,
    pub platform_get_window_minimized:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport) -> bool>,
    pub platform_set_window_title:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, *const c_char)>,
    pub platform_set_window_alpha:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, f32)>,
    pub platform_update_window: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport)>,
    pub platform_render_window:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, *mut c_void)>,
    pub platform_swap_buffers:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, *mut c_void)>,
    pub platform_get_window_dpi_scale: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport) -> f32>,
    pub platform_on_changed_viewport: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport)>,
    pub platform_get_window_work_area_insets:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport) -> sys::ImVec4_c>,
    pub platform_create_vk_surface: Option<
        unsafe extern "C" fn(
            *mut sys::ImGuiViewport,
            sys::ImU64,
            *const c_void,
            *mut sys::ImU64,
        ) -> i32,
    >,
    pub renderer_create_window: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport)>,
    pub renderer_destroy_window: Option<unsafe extern "C" fn(*mut sys::ImGuiViewport)>,
    pub renderer_set_window_size:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, sys::ImVec2_c)>,
    pub renderer_render_window:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, *mut c_void)>,
    pub renderer_swap_buffers:
        Option<unsafe extern "C" fn(*mut sys::ImGuiViewport, *mut c_void)>,
    pub monitors: sys::ImVector_ImGuiPlatformMonitor,
    pub textures: sys::ImVector_ImTextureDataPtr,
    pub viewports: sys::ImVector_ImGuiViewportPtr,
}

#[cfg(not(feature = "docking"))]
#[repr(C)]
pub struct PlatformIo {
    pub(crate) get_clipboard_text_fn:
        Option<unsafe extern "C" fn(*mut sys::ImGuiContext) -> *const c_char>,
    pub(crate) set_clipboard_text_fn:
        Option<unsafe extern "C" fn(*mut sys::ImGuiContext, *const c_char)>,
    pub(crate) clipboard_user_data: *mut c_void,
    pub(crate) open_in_shell_fn:
        Option<unsafe extern "C" fn(ctx: *mut sys::ImGuiContext, path: *const c_char) -> bool>,
    pub(crate) open_in_shell_user_data: *mut c_void,
    pub(crate) set_ime_data_fn: Option<
        unsafe extern "C" fn(
            ctx: *mut sys::ImGuiContext,
            viewport: *mut sys::ImGuiViewport,
            data: *mut sys::ImGuiPlatformImeData,
        ),
    >,
    pub(crate) ime_user_data: *mut c_void,
    pub(crate) locale_decimal_point: sys::ImWchar,
    platform_session_date: i32,
    pub renderer_texture_max_width: i32,
    pub renderer_texture_max_height: i32,
    pub renderer_render_state: *mut c_void,
    draw_callback_reset_render_state: sys::ImDrawCallback,
    draw_callback_set_sampler_linear: sys::ImDrawCallback,
    draw_callback_set_sampler_nearest: sys::ImDrawCallback,
    pub textures: sys::ImVector_ImTextureDataPtr,
}

unsafe impl RawCast<sys::ImGuiPlatformIO> for PlatformIo {}

#[test]
#[cfg(all(test, feature = "docking"))]
fn test_platform_io_memory_layout() {
    use std::mem;
    assert_eq!(
        mem::size_of::<PlatformIo>(),
        mem::size_of::<sys::ImGuiPlatformIO>()
    );
    assert_eq!(
        mem::align_of::<PlatformIo>(),
        mem::align_of::<sys::ImGuiPlatformIO>()
    );
    use sys::ImGuiPlatformIO;
    macro_rules! assert_field_offset {
        ($l:ident, $r:ident) => {
            assert_eq!(
                memoffset::offset_of!(PlatformIo, $l),
                memoffset::offset_of!(ImGuiPlatformIO, $r)
            );
        };
    }

    assert_field_offset!(get_clipboard_text_fn, Platform_GetClipboardTextFn);
    assert_field_offset!(set_clipboard_text_fn, Platform_SetClipboardTextFn);
    assert_field_offset!(clipboard_user_data, Platform_ClipboardUserData);
    assert_field_offset!(open_in_shell_fn, Platform_OpenInShellFn);
    assert_field_offset!(open_in_shell_user_data, Platform_OpenInShellUserData);
    assert_field_offset!(set_ime_data_fn, Platform_SetImeDataFn);
    assert_field_offset!(ime_user_data, Platform_ImeUserData);
    assert_field_offset!(locale_decimal_point, Platform_LocaleDecimalPoint);
    assert_field_offset!(renderer_texture_max_width, Renderer_TextureMaxWidth);
    assert_field_offset!(renderer_texture_max_height, Renderer_TextureMaxHeight);
    assert_field_offset!(renderer_render_state, Renderer_RenderState);
    assert_field_offset!(platform_create_window, Platform_CreateWindow);
    assert_field_offset!(platform_destroy_window, Platform_DestroyWindow);
    assert_field_offset!(platform_show_window, Platform_ShowWindow);
    assert_field_offset!(platform_set_window_pos, Platform_SetWindowPos);
    assert_field_offset!(platform_get_window_pos, Platform_GetWindowPos);
    assert_field_offset!(platform_set_window_size, Platform_SetWindowSize);
    assert_field_offset!(platform_get_window_size, Platform_GetWindowSize);
    assert_field_offset!(
        platform_get_window_framebuffer_scale,
        Platform_GetWindowFramebufferScale
    );
    assert_field_offset!(platform_set_window_focus, Platform_SetWindowFocus);
    assert_field_offset!(platform_get_window_focus, Platform_GetWindowFocus);
    assert_field_offset!(platform_get_window_minimized, Platform_GetWindowMinimized);
    assert_field_offset!(platform_set_window_title, Platform_SetWindowTitle);
    assert_field_offset!(platform_set_window_alpha, Platform_SetWindowAlpha);
    assert_field_offset!(platform_update_window, Platform_UpdateWindow);
    assert_field_offset!(platform_render_window, Platform_RenderWindow);
    assert_field_offset!(platform_swap_buffers, Platform_SwapBuffers);
    assert_field_offset!(platform_get_window_dpi_scale, Platform_GetWindowDpiScale);
    assert_field_offset!(platform_on_changed_viewport, Platform_OnChangedViewport);
    assert_field_offset!(
        platform_get_window_work_area_insets,
        Platform_GetWindowWorkAreaInsets
    );
    assert_field_offset!(platform_create_vk_surface, Platform_CreateVkSurface);
    assert_field_offset!(renderer_create_window, Renderer_CreateWindow);
    assert_field_offset!(renderer_destroy_window, Renderer_DestroyWindow);
    assert_field_offset!(renderer_set_window_size, Renderer_SetWindowSize);
    assert_field_offset!(renderer_render_window, Renderer_RenderWindow);
    assert_field_offset!(renderer_swap_buffers, Renderer_SwapBuffers);
    assert_field_offset!(monitors, Monitors);
    assert_field_offset!(textures, Textures);
    assert_field_offset!(viewports, Viewports);
}

#[test]
#[cfg(all(test, not(feature = "docking")))]
fn test_platform_io_memory_layout() {
    use std::mem;
    assert_eq!(
        mem::size_of::<PlatformIo>(),
        mem::size_of::<sys::ImGuiPlatformIO>()
    );
    assert_eq!(
        mem::align_of::<PlatformIo>(),
        mem::align_of::<sys::ImGuiPlatformIO>()
    );
    use sys::ImGuiPlatformIO;
    macro_rules! assert_field_offset {
        ($l:ident, $r:ident) => {
            assert_eq!(
                memoffset::offset_of!(PlatformIo, $l),
                memoffset::offset_of!(ImGuiPlatformIO, $r)
            );
        };
    }

    assert_field_offset!(get_clipboard_text_fn, Platform_GetClipboardTextFn);
    assert_field_offset!(set_clipboard_text_fn, Platform_SetClipboardTextFn);
    assert_field_offset!(clipboard_user_data, Platform_ClipboardUserData);
    assert_field_offset!(open_in_shell_fn, Platform_OpenInShellFn);
    assert_field_offset!(open_in_shell_user_data, Platform_OpenInShellUserData);
    assert_field_offset!(set_ime_data_fn, Platform_SetImeDataFn);
    assert_field_offset!(ime_user_data, Platform_ImeUserData);
    assert_field_offset!(locale_decimal_point, Platform_LocaleDecimalPoint);
    assert_field_offset!(renderer_texture_max_width, Renderer_TextureMaxWidth);
        assert_field_offset!(renderer_texture_max_height, Renderer_TextureMaxHeight);
        assert_field_offset!(renderer_render_state, Renderer_RenderState);
        assert_field_offset!(draw_callback_reset_render_state, DrawCallback_ResetRenderState);
        assert_field_offset!(draw_callback_set_sampler_linear, DrawCallback_SetSamplerLinear);
        assert_field_offset!(draw_callback_set_sampler_nearest, DrawCallback_SetSamplerNearest);
        assert_field_offset!(textures, Textures);
}

/// Describes an ImGui Viewport.
#[cfg(feature = "docking")]
#[repr(C)]
pub struct Viewport {
    /// The unique ID of this Viewport.
    pub id: crate::Id,
    /// Flags that describe how the Viewport should behave.
    pub flags: crate::ViewportFlags,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub framebuffer_scale: [f32; 2],
    pub work_pos: [f32; 2],
    pub work_size: [f32; 2],
    pub dpi_scale: f32,
    pub parent_viewport_id: crate::Id,
    pub parent_viewport: *mut sys::ImGuiViewport,
    pub draw_data: *mut sys::ImDrawData,
    pub renderer_user_data: *mut c_void,
    pub platform_user_data: *mut c_void,
    pub platform_icon_data: *mut c_void,
    pub platform_handle: *mut c_void,
    pub platform_handle_raw: *mut c_void,
    pub platform_window_created: bool,
    pub platform_request_move: bool,
    pub platform_request_resize: bool,
    pub platform_request_close: bool,
}

#[cfg(not(feature = "docking"))]
#[repr(C)]
pub struct Viewport {
    /// The unique ID of this Viewport.
    pub id: crate::Id,
    /// Flags that describe how the Viewport should behave.
    pub flags: crate::ViewportFlags,
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub framebuffer_scale: [f32; 2],
    pub work_pos: [f32; 2],
    pub work_size: [f32; 2],
    pub platform_handle: *mut c_void,
    pub platform_handle_raw: *mut c_void,
}

#[test]
#[cfg(all(test, feature = "docking"))]
fn test_viewport_memory_layout() {
    use std::mem;
    assert_eq!(
        mem::size_of::<Viewport>(),
        mem::size_of::<sys::ImGuiViewport>()
    );
    assert_eq!(
        mem::align_of::<Viewport>(),
        mem::align_of::<sys::ImGuiViewport>()
    );
    use sys::ImGuiViewport;
    macro_rules! assert_field_offset {
        ($l:ident, $r:ident) => {
            assert_eq!(
                memoffset::offset_of!(Viewport, $l),
                memoffset::offset_of!(ImGuiViewport, $r)
            );
        };
    }

    assert_field_offset!(id, ID);
    assert_field_offset!(flags, Flags);
    assert_field_offset!(pos, Pos);
    assert_field_offset!(size, Size);
    assert_field_offset!(framebuffer_scale, FramebufferScale);
    assert_field_offset!(work_pos, WorkPos);
    assert_field_offset!(work_size, WorkSize);
    assert_field_offset!(dpi_scale, DpiScale);
    assert_field_offset!(parent_viewport_id, ParentViewportId);
    assert_field_offset!(parent_viewport, ParentViewport);
    assert_field_offset!(draw_data, DrawData);
    assert_field_offset!(renderer_user_data, RendererUserData);
    assert_field_offset!(platform_user_data, PlatformUserData);
    assert_field_offset!(platform_icon_data, PlatformIconData);
    assert_field_offset!(platform_handle, PlatformHandle);
    assert_field_offset!(platform_handle_raw, PlatformHandleRaw);
    assert_field_offset!(platform_window_created, PlatformWindowCreated);
    assert_field_offset!(platform_request_move, PlatformRequestMove);
    assert_field_offset!(platform_request_resize, PlatformRequestResize);
    assert_field_offset!(platform_request_close, PlatformRequestClose);
}

#[test]
#[cfg(all(test, not(feature = "docking")))]
fn test_viewport_memory_layout() {
    use std::mem;
    assert_eq!(
        mem::size_of::<Viewport>(),
        mem::size_of::<sys::ImGuiViewport>()
    );
    assert_eq!(
        mem::align_of::<Viewport>(),
        mem::align_of::<sys::ImGuiViewport>()
    );
    use sys::ImGuiViewport;
    macro_rules! assert_field_offset {
        ($l:ident, $r:ident) => {
            assert_eq!(
                memoffset::offset_of!(Viewport, $l),
                memoffset::offset_of!(ImGuiViewport, $r)
            );
        };
    }

    assert_field_offset!(id, ID);
    assert_field_offset!(flags, Flags);
    assert_field_offset!(pos, Pos);
    assert_field_offset!(size, Size);
    assert_field_offset!(framebuffer_scale, FramebufferScale);
    assert_field_offset!(work_pos, WorkPos);
    assert_field_offset!(work_size, WorkSize);
    assert_field_offset!(platform_handle, PlatformHandle);
    assert_field_offset!(platform_handle_raw, PlatformHandleRaw);
}
