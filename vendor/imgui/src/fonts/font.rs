use crate::fonts::atlas::{FontAtlas, FontId};
use crate::internal::RawCast;
use crate::sys;

/// Runtime data for a single font within a font atlas
#[repr(C)]
pub struct Font {
    pub last_baked: *mut sys::ImFontBaked,
    pub owner_atlas: *mut FontAtlas,
    pub flags: sys::ImFontFlags,
    pub current_rasterizer_density: f32,
    pub font_id: sys::ImGuiID,
    pub legacy_size: f32,
    pub sources: sys::ImVector_ImFontConfigPtr,
    pub ellipsis_char: sys::ImWchar,
    pub fallback_char: sys::ImWchar,
    pub used_8k_pages_map: [sys::ImU8; 17],
    pub ellipsis_auto_bake: bool,
    pub remap_pairs: sys::ImGuiStorage,
}

unsafe impl RawCast<sys::ImFont> for Font {}

impl Font {
    /// Returns the identifier of this font
    pub fn id(&self) -> FontId {
        FontId(self as *const _)
    }
}

#[test]
fn test_font_memory_layout() {
    use std::mem;
    assert_eq!(mem::size_of::<Font>(), mem::size_of::<sys::ImFont>());
    assert_eq!(mem::align_of::<Font>(), mem::align_of::<sys::ImFont>());
    use sys::ImFont;
    macro_rules! assert_field_offset {
        ($l:ident, $r:ident) => {
            assert_eq!(
                memoffset::offset_of!(Font, $l),
                memoffset::offset_of!(ImFont, $r)
            );
        };
    }

    assert_field_offset!(last_baked, LastBaked);
    assert_field_offset!(owner_atlas, OwnerAtlas);
    assert_field_offset!(flags, Flags);
    assert_field_offset!(current_rasterizer_density, CurrentRasterizerDensity);
    assert_field_offset!(font_id, FontId);
    assert_field_offset!(legacy_size, LegacySize);
    assert_field_offset!(sources, Sources);
    assert_field_offset!(ellipsis_char, EllipsisChar);
    assert_field_offset!(fallback_char, FallbackChar);
    assert_field_offset!(used_8k_pages_map, Used8kPagesMap);
    assert_field_offset!(ellipsis_auto_bake, EllipsisAutoBake);
    assert_field_offset!(remap_pairs, RemapPairs);
}
