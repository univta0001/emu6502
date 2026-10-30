use bitflags::bitflags;
use std::f32;
use std::os::raw::{c_char, c_int, c_uchar, c_uint, c_void};
use std::rc::Rc;
use std::slice;

use crate::fonts::font::Font;
use crate::fonts::glyph_ranges::FontGlyphRanges;
use crate::internal::{ImVector, RawCast};
use crate::sys;

bitflags! {
    /// Font atlas configuration flags
    #[repr(transparent)]
    pub struct FontAtlasFlags: i32 {
        /// Don't round the height to next power of two
        const NO_POWER_OF_TWO_HEIGHT = sys::ImFontAtlasFlags_NoPowerOfTwoHeight as i32;
        /// Don't build software mouse cursors into the atlas
        const NO_MOUSE_CURSORS = sys::ImFontAtlasFlags_NoMouseCursors as i32;
        /// Don't build thick line textures into the atlas
        const NO_BAKED_LINES = sys::ImFontAtlasFlags_NoBakedLines as i32;
    }
}

/// A font identifier
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct FontId(pub(crate) *const Font);

/// A font atlas that builds a single texture
#[repr(C)]
pub struct FontAtlas {
    /// Configuration flags
    pub flags: FontAtlasFlags,
    /// Texture format desired by user before building the atlas.
    pub tex_desired_format: sys::ImTextureFormat,
    /// Padding between glyphs within texture in pixels.
    ///
    /// Defaults to 1. If your rendering method doesn't rely on bilinear filtering, you may set
    /// this to 0.
    pub tex_glyph_padding: i32,
    /// Minimum texture width before building the atlas.
    pub tex_min_width: i32,
    /// Minimum texture height before building the atlas.
    pub tex_min_height: i32,
    /// Maximum texture width allowed by renderer (if any).
    pub tex_max_width: i32,
    /// Maximum texture height allowed by renderer (if any).
    pub tex_max_height: i32,

    user_data: *mut c_void,
    pub tex_ref: sys::ImTextureRef,
    tex_data: *mut sys::ImTextureData,
    tex_list: sys::ImVector_ImTextureDataPtr,
    locked: bool,
    renderer_has_textures: bool,
    tex_is_built: bool,
    tex_pixels_use_colors: bool,
    tex_uv_scale: sys::ImVec2,
    tex_uv_white_pixel: sys::ImVec2,
    fonts: ImVector<*mut Font>,
    sources: sys::ImVector_ImFontConfig,
    tex_uv_lines: [sys::ImVec4; 33],
    tex_next_unique_id: i32,
    font_next_unique_id: i32,
    draw_list_shared_datas: sys::ImVector_ImDrawListSharedDataPtr,
    builder: *mut sys::ImFontAtlasBuilder,
    font_loader: *const sys::ImFontLoader,
    font_loader_name: *const c_char,
    font_loader_data: *mut c_void,
    font_loader_flags: c_uint,
    ref_count: i32,
    owner_context: *mut sys::ImGuiContext,
}

unsafe impl RawCast<sys::ImFontAtlas> for FontAtlas {}

impl FontAtlas {
    #[doc(alias = "AddFontDefault", alias = "AddFont")]
    pub fn add_font(&mut self, font_sources: &[FontSource<'_>]) -> FontId {
        let (head, tail) = font_sources.split_first().unwrap();
        let font_id = self.add_font_internal(head, false);
        for font in tail {
            self.add_font_internal(font, true);
        }
        font_id
    }
    fn add_font_internal(&mut self, font_source: &FontSource<'_>, merge_mode: bool) -> FontId {
        let mut raw_config = sys_font_config_default();
        raw_config.MergeMode = merge_mode;
        let raw_font = match font_source {
            FontSource::DefaultFontData { config } => unsafe {
                if let Some(config) = config {
                    config.apply_to_raw_config(&mut raw_config, self.raw_mut());
                }
                sys::ImFontAtlas_AddFontDefault(self.raw_mut(), &raw_config)
            },
            FontSource::TtfData {
                data,
                size_pixels,
                config,
            } => {
                if let Some(config) = config {
                    unsafe {
                        config.apply_to_raw_config(&mut raw_config, self.raw_mut());
                    }
                }
                // We can't guarantee `data` is alive when the font atlas is built, so
                // make a copy and move ownership of the data to the atlas
                let data_copy = unsafe {
                    let ptr = sys::igMemAlloc(data.len()) as *mut u8;
                    assert!(!ptr.is_null());
                    slice::from_raw_parts_mut(ptr, data.len())
                };
                data_copy.copy_from_slice(data);
                raw_config.FontData = data_copy.as_mut_ptr() as *mut c_void;
                raw_config.FontDataSize = data_copy.len() as i32;
                raw_config.FontDataOwnedByAtlas = true;
                raw_config.SizePixels = *size_pixels;
                unsafe { sys::ImFontAtlas_AddFont(self.raw_mut(), &raw_config) }
            }
        };
        FontId(raw_font as *const _)
    }
    pub fn fonts(&self) -> Vec<FontId> {
        let mut result = Vec::new();
        unsafe {
            for &font in self.fonts.as_slice() {
                result.push((*font).id());
            }
        }
        result
    }
    pub fn get_font(&self, id: FontId) -> Option<&Font> {
        unsafe {
            for &font in self.fonts.as_slice() {
                if id == FontId(font) {
                    return Some(&*(font as *const Font));
                }
            }
        }
        None
    }
    /// Returns true if the font atlas has been built
    #[doc(alias = "IsBuilt")]
    pub fn is_built(&self) -> bool {
        self.tex_is_built && !self.fonts.as_slice().is_empty()
    }
    /// Builds a 1 byte per-pixel font atlas texture
    #[doc(alias = "GetTextDataAsAlpha8")]
    pub fn build_alpha8_texture(&mut self) -> FontAtlasTexture<'_> {
        self.build_texture_with_bytes_per_pixel(1)
    }
    /// Builds a 4 byte per-pixel font atlas texture
    #[doc(alias = "GetTextDataAsRGBA32")]
    pub fn build_rgba32_texture(&mut self) -> FontAtlasTexture<'_> {
        self.build_texture_with_bytes_per_pixel(4)
    }

    fn build_texture_with_bytes_per_pixel(&self, expected_bpp: c_int) -> FontAtlasTexture<'_> {
        let tex_data = self.tex_data;
        assert!(
            !tex_data.is_null(),
            "font texture data is not available yet"
        );
        unsafe {
            let tex_data = &*tex_data;
            let width = tex_data.Width;
            let height = tex_data.Height;
            let bytes_per_pixel = tex_data.BytesPerPixel;
            let pixels = tex_data.Pixels as *const c_uchar;

            assert!(width >= 0, "font texture width must be positive");
            assert!(height >= 0, "font texture height must be positive");
            assert!(
                bytes_per_pixel >= 0,
                "font texture bytes per pixel must be positive"
            );
            assert_eq!(
                bytes_per_pixel, expected_bpp,
                "unexpected font texture format"
            );

            let height = height as usize;
            // Check multiplication to avoid constructing an invalid slice in case of overflow
            let pitch = width
                .checked_mul(bytes_per_pixel)
                .expect("Overflow in font texture pitch calculation")
                as usize;
            FontAtlasTexture {
                width: width as u32,
                height: height as u32,
                data: slice::from_raw_parts(pixels, pitch * height),
            }
        }
    }
    /// Clears the font atlas completely (both input and output data)
    #[doc(alias = "Clear")]
    pub fn clear(&mut self) {
        unsafe {
            sys::ImFontAtlas_Clear(self.raw_mut());
        }
    }
    /// Clears output font data (glyph storage, UV coordinates)
    #[doc(alias = "ClearFonts")]
    pub fn clear_fonts(&mut self) {
        unsafe {
            sys::ImFontAtlas_ClearFonts(self.raw_mut());
        }
    }
    /// Clears output texture data.
    ///
    /// Can be used to save RAM once the texture has been transferred to the GPU.
    #[doc(alias = "ClearTexData")]
    pub fn clear_tex_data(&mut self) {
        unsafe {
            sys::ImFontAtlas_ClearTexData(self.raw_mut());
        }
    }
    /// Clears all the data used to build the textures and fonts
    #[doc(alias = "ClearInputData")]
    pub fn clear_input_data(&mut self) {
        unsafe {
            sys::ImFontAtlas_ClearInputData(self.raw_mut());
        }
    }
}

#[test]
#[cfg(test)]
fn test_font_atlas_memory_layout() {
    use std::mem;
    assert_eq!(
        mem::size_of::<FontAtlas>(),
        mem::size_of::<sys::ImFontAtlas>()
    );
    assert_eq!(
        mem::align_of::<FontAtlas>(),
        mem::align_of::<sys::ImFontAtlas>()
    );
    use sys::ImFontAtlas;
    macro_rules! assert_field_offset {
        ($l:ident, $r:ident) => {
            assert_eq!(
                memoffset::offset_of!(FontAtlas, $l),
                memoffset::offset_of!(ImFontAtlas, $r)
            );
        };
    }
    assert_field_offset!(flags, Flags);
    assert_field_offset!(tex_desired_format, TexDesiredFormat);
    assert_field_offset!(tex_glyph_padding, TexGlyphPadding);
    assert_field_offset!(tex_min_width, TexMinWidth);
    assert_field_offset!(tex_min_height, TexMinHeight);
    assert_field_offset!(tex_max_width, TexMaxWidth);
    assert_field_offset!(tex_max_height, TexMaxHeight);
    assert_field_offset!(user_data, UserData);
    assert_field_offset!(tex_ref, TexRef);
    assert_field_offset!(tex_data, TexData);
    assert_field_offset!(tex_list, TexList);
    assert_field_offset!(locked, Locked);
    assert_field_offset!(renderer_has_textures, RendererHasTextures);
    assert_field_offset!(tex_is_built, TexIsBuilt);
    assert_field_offset!(tex_pixels_use_colors, TexPixelsUseColors);
    assert_field_offset!(tex_uv_scale, TexUvScale);
    assert_field_offset!(tex_uv_white_pixel, TexUvWhitePixel);
    assert_field_offset!(fonts, Fonts);
    assert_field_offset!(sources, Sources);
    assert_field_offset!(tex_uv_lines, TexUvLines);
    assert_field_offset!(tex_next_unique_id, TexNextUniqueID);
    assert_field_offset!(font_next_unique_id, FontNextUniqueID);
    assert_field_offset!(draw_list_shared_datas, DrawListSharedDatas);
    assert_field_offset!(builder, Builder);
    assert_field_offset!(font_loader, FontLoader);
    assert_field_offset!(font_loader_name, FontLoaderName);
    assert_field_offset!(font_loader_data, FontLoaderData);
    assert_field_offset!(font_loader_flags, FontLoaderFlags);
    assert_field_offset!(ref_count, RefCount);
    assert_field_offset!(owner_context, OwnerContext);
}

/// A source for binary font data
#[derive(Clone, Debug)]
pub enum FontSource<'a> {
    /// Default font included with the library (ProggyClean.ttf)
    DefaultFontData { config: Option<FontConfig> },
    /// Binary TTF/OTF font data
    TtfData {
        data: &'a [u8],
        size_pixels: f32,
        config: Option<FontConfig>,
    },
}

/// Configuration settings for a font
#[derive(Clone, Debug)]
pub struct FontConfig {
    /// Size in pixels for the rasterizer
    pub size_pixels: f32,
    /// Horizontal oversampling
    pub oversample_h: i8,
    /// Vertical oversampling
    pub oversample_v: i8,
    /// Align every glyph to pixel boundary
    pub pixel_snap_h: bool,
    /// Offset for all glyphs in this font
    pub glyph_offset: [f32; 2],
    /// Unicode ranges to use from this font
    pub glyph_ranges: FontGlyphRanges,
    /// Unicode ranges to exclude from this font
    pub glyph_exclude_ranges: Option<FontGlyphRanges>,
    /// Minimum advance_x for glyphs
    pub glyph_min_advance_x: f32,
    /// Maximum advance_x for glyphs
    pub glyph_max_advance_x: f32,
    /// Extra advance between glyphs
    pub glyph_extra_advance_x: f32,
    /// Font index in the font file
    pub font_no: u32,
    /// Settings for a custom font loader if used
    pub font_loader_flags: u32,
    /// Brighten (>1.0) or darken (<1.0) font output
    pub rasterizer_multiply: f32,
    /// DPI scale for rasterization, not altering other font metrics:
    /// make it easy to swap between e.g. a 100% and a 400% fonts for a zooming display.
    /// IMPORTANT: If you increase this it is expected that you increase font scale
    /// accordingly, otherwise quality may look lowered.
    pub rasterizer_density: f32,
    /// Font flags
    pub flags: sys::ImFontFlags,
    /// Explicitly specify the ellipsis character.
    ///
    /// With multiple font sources the first specified ellipsis is used.
    pub ellipsis_char: Option<char>,
    pub name: Option<String>,
}

impl Default for FontConfig {
    fn default() -> FontConfig {
        let sys_font_config = sys_font_config_default();
        FontConfig {
            size_pixels: sys_font_config.SizePixels,
            oversample_h: sys_font_config.OversampleH,
            oversample_v: sys_font_config.OversampleV,
            pixel_snap_h: sys_font_config.PixelSnapH,
            glyph_offset: [sys_font_config.GlyphOffset.x, sys_font_config.GlyphOffset.y],
            glyph_ranges: FontGlyphRanges::default(),
            glyph_exclude_ranges: None,
            glyph_min_advance_x: sys_font_config.GlyphMinAdvanceX,
            glyph_max_advance_x: sys_font_config.GlyphMaxAdvanceX,
            glyph_extra_advance_x: sys_font_config.GlyphExtraAdvanceX,
            font_no: sys_font_config.FontNo,
            font_loader_flags: sys_font_config.FontLoaderFlags,
            rasterizer_multiply: sys_font_config.RasterizerMultiply,
            rasterizer_density: sys_font_config.RasterizerDensity,
            flags: sys_font_config.Flags,
            ellipsis_char: match sys_font_config.EllipsisChar {
                0 => None,
                value => std::char::from_u32(value as u32),
            },
            name: None,
        }
    }
}

impl FontConfig {
    fn apply_to_raw_config(&self, raw: &mut sys::ImFontConfig, atlas: *mut sys::ImFontAtlas) {
        raw.SizePixels = self.size_pixels;
        raw.OversampleH = self.oversample_h;
        raw.OversampleV = self.oversample_v;
        raw.PixelSnapH = self.pixel_snap_h;
        raw.GlyphOffset = self.glyph_offset.into();
        raw.GlyphRanges = unsafe { self.glyph_ranges.to_ptr(atlas) };
        raw.GlyphExcludeRanges = match self.glyph_exclude_ranges.as_ref() {
            Some(ranges) => unsafe { ranges.to_ptr(atlas) },
            None => std::ptr::null(),
        };
        raw.GlyphMinAdvanceX = self.glyph_min_advance_x;
        raw.GlyphMaxAdvanceX = self.glyph_max_advance_x;
        raw.GlyphExtraAdvanceX = self.glyph_extra_advance_x;
        raw.FontNo = self.font_no;
        raw.FontLoaderFlags = self.font_loader_flags;
        raw.RasterizerMultiply = self.rasterizer_multiply;
        raw.RasterizerDensity = self.rasterizer_density;
        raw.Flags = self.flags;
        raw.EllipsisChar = self.ellipsis_char.unwrap_or('\0') as sys::ImWchar;
        if let Some(name) = self.name.as_ref() {
            let bytes = name.as_bytes();
            let mut len = bytes.len().min(raw.Name.len() - 1);
            while !name.is_char_boundary(len) {
                len -= 1;
            }
            unsafe {
                bytes.as_ptr().copy_to(raw.Name.as_mut_ptr() as _, len);
                raw.Name[len] = 0;
            }
        }
    }
}

fn sys_font_config_default() -> sys::ImFontConfig {
    unsafe {
        let heap_allocated = sys::ImFontConfig_ImFontConfig();
        let copy = *heap_allocated;
        sys::ImFontConfig_destroy(heap_allocated);
        copy
    }
}

#[test]
fn test_font_config_default() {
    let sys_font_config = sys_font_config_default();
    let font_config = FontConfig::default();
    assert_eq!(font_config.size_pixels, sys_font_config.SizePixels);
    assert_eq!(font_config.oversample_h, sys_font_config.OversampleH);
    assert_eq!(font_config.oversample_v, sys_font_config.OversampleV);
    assert_eq!(font_config.pixel_snap_h, sys_font_config.PixelSnapH);
    assert_eq!(font_config.glyph_offset[0], sys_font_config.GlyphOffset.x);
    assert_eq!(font_config.glyph_offset[1], sys_font_config.GlyphOffset.y);
    assert_eq!(
        font_config.glyph_min_advance_x,
        sys_font_config.GlyphMinAdvanceX
    );
    assert_eq!(
        font_config.glyph_max_advance_x,
        sys_font_config.GlyphMaxAdvanceX
    );
    assert_eq!(
        font_config.glyph_extra_advance_x,
        sys_font_config.GlyphExtraAdvanceX
    );
    assert_eq!(font_config.font_no, sys_font_config.FontNo);
    assert_eq!(
        font_config.font_loader_flags,
        sys_font_config.FontLoaderFlags
    );
    assert_eq!(
        font_config.rasterizer_multiply,
        sys_font_config.RasterizerMultiply
    );
}

/// Handle to a font atlas texture
#[derive(Clone, Debug)]
pub struct FontAtlasTexture<'a> {
    /// Texture width (in pixels)
    pub width: u32,
    /// Texture height (in pixels)
    pub height: u32,
    /// Raw texture data (in bytes).
    ///
    /// The format depends on which function was called to obtain this data.
    pub data: &'a [u8],
}

/// A font atlas that can be shared between contexts
#[derive(Debug, Clone)]
pub struct SharedFontAtlas(pub(crate) Rc<*mut sys::ImFontAtlas>);

impl std::ops::Deref for SharedFontAtlas {
    type Target = Rc<*mut sys::ImFontAtlas>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SharedFontAtlas {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl SharedFontAtlas {
    #[doc(alias = "ImFontAtlas", alias = "ImFontAtlas::ImFontAtlas")]
    pub fn create() -> SharedFontAtlas {
        SharedFontAtlas(unsafe { Rc::new(sys::ImFontAtlas_ImFontAtlas()) })
    }

    /// Gets a raw pointer to the underlying `ImFontAtlas`.
    pub fn as_ptr(&self) -> *const sys::ImFontAtlas {
        *self.0 as *const _
    }

    /// Gets a raw pointer to the underlying `ImFontAtlas`.
    pub fn as_ptr_mut(&mut self) -> *mut sys::ImFontAtlas {
        *self.0
    }
}

impl Drop for SharedFontAtlas {
    #[doc(alias = "ImFontAtlas::Destory")]
    fn drop(&mut self) {
        // ImGui owns shared font atlases and releases them when the last context shuts down.
    }
}

// /// An immutably borrowed reference to a (possibly shared) font atlas
// pub enum FontAtlasRef<'a> {
//     Owned(&'a FontAtlas),
//     Shared(&'a cell::RefMut<'a, SharedFontAtlas>),
// }

// impl<'a> Deref for FontAtlasRef<'a> {
//     type Target = FontAtlas;
//     fn deref(&self) -> &FontAtlas {
//         use self::FontAtlasRef::*;
//         match self {
//             Owned(atlas) => atlas,
//             Shared(cell) => {
//                 let font_atlas: &SharedFontAtlas = &cell;
//                 let font_atlas: &FontAtlas = &font_atlas;

//                 todo!()
//             }
//         }
//     }
// }

// /// A mutably borrowed reference to a (possibly shared) font atlas
// pub enum FontAtlasRefMut<'a> {
//     Owned(&'a mut FontAtlas),
//     Shared(cell::RefMut<'a, SharedFontAtlas>),
// }

// impl<'a> Deref for FontAtlasRefMut<'a> {
//     type Target = FontAtlas;
//     fn deref(&self) -> &FontAtlas {
//         use self::FontAtlasRefMut::*;
//         match self {
//             Owned(atlas) => atlas,
//             Shared(cell) => cell,
//         }
//     }
// }

// impl<'a> DerefMut for FontAtlasRefMut<'a> {
//     fn deref_mut(&mut self) -> &mut FontAtlas {
//         use self::FontAtlasRefMut::*;
//         match self {
//             Owned(atlas) => atlas,
//             Shared(cell) => cell,
//         }
//     }
// }
