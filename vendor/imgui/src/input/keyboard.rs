use crate::sys;
use crate::Ui;

/// A key identifier
#[repr(i32)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[allow(missing_docs)] // Self-describing
#[non_exhaustive]
pub enum Key {
    Tab = sys::ImGuiKey_Tab as i32,
    LeftArrow = sys::ImGuiKey_LeftArrow as i32,
    RightArrow = sys::ImGuiKey_RightArrow as i32,
    UpArrow = sys::ImGuiKey_UpArrow as i32,
    DownArrow = sys::ImGuiKey_DownArrow as i32,
    PageUp = sys::ImGuiKey_PageUp as i32,
    PageDown = sys::ImGuiKey_PageDown as i32,
    Home = sys::ImGuiKey_Home as i32,
    End = sys::ImGuiKey_End as i32,
    Insert = sys::ImGuiKey_Insert as i32,
    Delete = sys::ImGuiKey_Delete as i32,
    Backspace = sys::ImGuiKey_Backspace as i32,
    Space = sys::ImGuiKey_Space as i32,
    Enter = sys::ImGuiKey_Enter as i32,
    Escape = sys::ImGuiKey_Escape as i32,
    LeftCtrl = sys::ImGuiKey_LeftCtrl as i32,
    LeftShift = sys::ImGuiKey_LeftShift as i32,
    LeftAlt = sys::ImGuiKey_LeftAlt as i32,
    LeftSuper = sys::ImGuiKey_LeftSuper as i32,
    RightCtrl = sys::ImGuiKey_RightCtrl as i32,
    RightShift = sys::ImGuiKey_RightShift as i32,
    RightAlt = sys::ImGuiKey_RightAlt as i32,
    RightSuper = sys::ImGuiKey_RightSuper as i32,
    Menu = sys::ImGuiKey_Menu as i32,
    Alpha0 = sys::ImGuiKey_0 as i32,
    Alpha1 = sys::ImGuiKey_1 as i32,
    Alpha2 = sys::ImGuiKey_2 as i32,
    Alpha3 = sys::ImGuiKey_3 as i32,
    Alpha4 = sys::ImGuiKey_4 as i32,
    Alpha5 = sys::ImGuiKey_5 as i32,
    Alpha6 = sys::ImGuiKey_6 as i32,
    Alpha7 = sys::ImGuiKey_7 as i32,
    Alpha8 = sys::ImGuiKey_8 as i32,
    Alpha9 = sys::ImGuiKey_9 as i32,
    A = sys::ImGuiKey_A as i32,
    B = sys::ImGuiKey_B as i32,
    C = sys::ImGuiKey_C as i32,
    D = sys::ImGuiKey_D as i32,
    E = sys::ImGuiKey_E as i32,
    F = sys::ImGuiKey_F as i32,
    G = sys::ImGuiKey_G as i32,
    H = sys::ImGuiKey_H as i32,
    I = sys::ImGuiKey_I as i32,
    J = sys::ImGuiKey_J as i32,
    K = sys::ImGuiKey_K as i32,
    L = sys::ImGuiKey_L as i32,
    M = sys::ImGuiKey_M as i32,
    N = sys::ImGuiKey_N as i32,
    O = sys::ImGuiKey_O as i32,
    P = sys::ImGuiKey_P as i32,
    Q = sys::ImGuiKey_Q as i32,
    R = sys::ImGuiKey_R as i32,
    S = sys::ImGuiKey_S as i32,
    T = sys::ImGuiKey_T as i32,
    U = sys::ImGuiKey_U as i32,
    V = sys::ImGuiKey_V as i32,
    W = sys::ImGuiKey_W as i32,
    X = sys::ImGuiKey_X as i32,
    Y = sys::ImGuiKey_Y as i32,
    Z = sys::ImGuiKey_Z as i32,
    F1 = sys::ImGuiKey_F1 as i32,
    F2 = sys::ImGuiKey_F2 as i32,
    F3 = sys::ImGuiKey_F3 as i32,
    F4 = sys::ImGuiKey_F4 as i32,
    F5 = sys::ImGuiKey_F5 as i32,
    F6 = sys::ImGuiKey_F6 as i32,
    F7 = sys::ImGuiKey_F7 as i32,
    F8 = sys::ImGuiKey_F8 as i32,
    F9 = sys::ImGuiKey_F9 as i32,
    F10 = sys::ImGuiKey_F10 as i32,
    F11 = sys::ImGuiKey_F11 as i32,
    F12 = sys::ImGuiKey_F12 as i32,
    F13 = sys::ImGuiKey_F13 as i32,
    F14 = sys::ImGuiKey_F14 as i32,
    F15 = sys::ImGuiKey_F15 as i32,
    F16 = sys::ImGuiKey_F16 as i32,
    F17 = sys::ImGuiKey_F17 as i32,
    F18 = sys::ImGuiKey_F18 as i32,
    F19 = sys::ImGuiKey_F19 as i32,
    F20 = sys::ImGuiKey_F20 as i32,
    F21 = sys::ImGuiKey_F21 as i32,
    F22 = sys::ImGuiKey_F22 as i32,
    F23 = sys::ImGuiKey_F23 as i32,
    F24 = sys::ImGuiKey_F24 as i32,
    Apostrophe = sys::ImGuiKey_Apostrophe as i32,
    Comma = sys::ImGuiKey_Comma as i32,
    Minus = sys::ImGuiKey_Minus as i32,
    Period = sys::ImGuiKey_Period as i32,
    Slash = sys::ImGuiKey_Slash as i32,
    Semicolon = sys::ImGuiKey_Semicolon as i32,
    Equal = sys::ImGuiKey_Equal as i32,
    LeftBracket = sys::ImGuiKey_LeftBracket as i32,
    Backslash = sys::ImGuiKey_Backslash as i32,
    RightBracket = sys::ImGuiKey_RightBracket as i32,
    GraveAccent = sys::ImGuiKey_GraveAccent as i32,
    CapsLock = sys::ImGuiKey_CapsLock as i32,
    ScrollLock = sys::ImGuiKey_ScrollLock as i32,
    NumLock = sys::ImGuiKey_NumLock as i32,
    PrintScreen = sys::ImGuiKey_PrintScreen as i32,
    Pause = sys::ImGuiKey_Pause as i32,
    Keypad0 = sys::ImGuiKey_Keypad0 as i32,
    Keypad1 = sys::ImGuiKey_Keypad1 as i32,
    Keypad2 = sys::ImGuiKey_Keypad2 as i32,
    Keypad3 = sys::ImGuiKey_Keypad3 as i32,
    Keypad4 = sys::ImGuiKey_Keypad4 as i32,
    Keypad5 = sys::ImGuiKey_Keypad5 as i32,
    Keypad6 = sys::ImGuiKey_Keypad6 as i32,
    Keypad7 = sys::ImGuiKey_Keypad7 as i32,
    Keypad8 = sys::ImGuiKey_Keypad8 as i32,
    Keypad9 = sys::ImGuiKey_Keypad9 as i32,
    KeypadDecimal = sys::ImGuiKey_KeypadDecimal as i32,
    KeypadDivide = sys::ImGuiKey_KeypadDivide as i32,
    KeypadMultiply = sys::ImGuiKey_KeypadMultiply as i32,
    KeypadSubtract = sys::ImGuiKey_KeypadSubtract as i32,
    KeypadAdd = sys::ImGuiKey_KeypadAdd as i32,
    KeypadEnter = sys::ImGuiKey_KeypadEnter as i32,
    KeypadEqual = sys::ImGuiKey_KeypadEqual as i32,
    AppBack = sys::ImGuiKey_AppBack as i32,
    AppForward = sys::ImGuiKey_AppForward as i32,
    Oem102 = sys::ImGuiKey_Oem102 as i32,
    GamepadStart = sys::ImGuiKey_GamepadStart as i32,
    GamepadBack = sys::ImGuiKey_GamepadBack as i32,
    GamepadFaceLeft = sys::ImGuiKey_GamepadFaceLeft as i32,
    GamepadFaceRight = sys::ImGuiKey_GamepadFaceRight as i32,
    GamepadFaceUp = sys::ImGuiKey_GamepadFaceUp as i32,
    GamepadFaceDown = sys::ImGuiKey_GamepadFaceDown as i32,
    GamepadDpadLeft = sys::ImGuiKey_GamepadDpadLeft as i32,
    GamepadDpadRight = sys::ImGuiKey_GamepadDpadRight as i32,
    GamepadDpadUp = sys::ImGuiKey_GamepadDpadUp as i32,
    GamepadDpadDown = sys::ImGuiKey_GamepadDpadDown as i32,
    GamepadL1 = sys::ImGuiKey_GamepadL1 as i32,
    GamepadR1 = sys::ImGuiKey_GamepadR1 as i32,
    GamepadL2 = sys::ImGuiKey_GamepadL2 as i32,
    GamepadR2 = sys::ImGuiKey_GamepadR2 as i32,
    GamepadL3 = sys::ImGuiKey_GamepadL3 as i32,
    GamepadR3 = sys::ImGuiKey_GamepadR3 as i32,
    GamepadLStickLeft = sys::ImGuiKey_GamepadLStickLeft as i32,
    GamepadLStickRight = sys::ImGuiKey_GamepadLStickRight as i32,
    GamepadLStickUp = sys::ImGuiKey_GamepadLStickUp as i32,
    GamepadLStickDown = sys::ImGuiKey_GamepadLStickDown as i32,
    GamepadRStickLeft = sys::ImGuiKey_GamepadRStickLeft as i32,
    GamepadRStickRight = sys::ImGuiKey_GamepadRStickRight as i32,
    GamepadRStickUp = sys::ImGuiKey_GamepadRStickUp as i32,
    GamepadRStickDown = sys::ImGuiKey_GamepadRStickDown as i32,
    MouseLeft = sys::ImGuiKey_MouseLeft as i32,
    MouseRight = sys::ImGuiKey_MouseRight as i32,
    MouseMiddle = sys::ImGuiKey_MouseMiddle as i32,
    MouseX1 = sys::ImGuiKey_MouseX1 as i32,
    MouseX2 = sys::ImGuiKey_MouseX2 as i32,
    MouseWheelX = sys::ImGuiKey_MouseWheelX as i32,
    MouseWheelY = sys::ImGuiKey_MouseWheelY as i32,
    ReservedForModCtrl = sys::ImGuiKey_ReservedForModCtrl as i32,
    ReservedForModShift = sys::ImGuiKey_ReservedForModShift as i32,
    ReservedForModAlt = sys::ImGuiKey_ReservedForModAlt as i32,
    ReservedForModSuper = sys::ImGuiKey_ReservedForModSuper as i32,

    ModCtrl = sys::ImGuiMod_Ctrl as i32,
    ModShift = sys::ImGuiMod_Shift as i32,
    ModAlt = sys::ImGuiMod_Alt as i32,
    ModSuper = sys::ImGuiMod_Super as i32,
}

impl Key {
    /// All possible `Key` variants
    pub const VARIANTS: [Key; Key::COUNT] = [
        Key::Tab,
        Key::LeftArrow,
        Key::RightArrow,
        Key::UpArrow,
        Key::DownArrow,
        Key::PageUp,
        Key::PageDown,
        Key::Home,
        Key::End,
        Key::Insert,
        Key::Delete,
        Key::Backspace,
        Key::Space,
        Key::Enter,
        Key::Escape,
        Key::LeftCtrl,
        Key::LeftShift,
        Key::LeftAlt,
        Key::LeftSuper,
        Key::RightCtrl,
        Key::RightShift,
        Key::RightAlt,
        Key::RightSuper,
        Key::Menu,
        Key::Alpha0,
        Key::Alpha1,
        Key::Alpha2,
        Key::Alpha3,
        Key::Alpha4,
        Key::Alpha5,
        Key::Alpha6,
        Key::Alpha7,
        Key::Alpha8,
        Key::Alpha9,
        Key::A,
        Key::B,
        Key::C,
        Key::D,
        Key::E,
        Key::F,
        Key::G,
        Key::H,
        Key::I,
        Key::J,
        Key::K,
        Key::L,
        Key::M,
        Key::N,
        Key::O,
        Key::P,
        Key::Q,
        Key::R,
        Key::S,
        Key::T,
        Key::U,
        Key::V,
        Key::W,
        Key::X,
        Key::Y,
        Key::Z,
        Key::F1,
        Key::F2,
        Key::F3,
        Key::F4,
        Key::F5,
        Key::F6,
        Key::F7,
        Key::F8,
        Key::F9,
        Key::F10,
        Key::F11,
        Key::F12,
        Key::F13,
        Key::F14,
        Key::F15,
        Key::F16,
        Key::F17,
        Key::F18,
        Key::F19,
        Key::F20,
        Key::F21,
        Key::F22,
        Key::F23,
        Key::F24,
        Key::Apostrophe,
        Key::Comma,
        Key::Minus,
        Key::Period,
        Key::Slash,
        Key::Semicolon,
        Key::Equal,
        Key::LeftBracket,
        Key::Backslash,
        Key::RightBracket,
        Key::GraveAccent,
        Key::CapsLock,
        Key::ScrollLock,
        Key::NumLock,
        Key::PrintScreen,
        Key::Pause,
        Key::Keypad0,
        Key::Keypad1,
        Key::Keypad2,
        Key::Keypad3,
        Key::Keypad4,
        Key::Keypad5,
        Key::Keypad6,
        Key::Keypad7,
        Key::Keypad8,
        Key::Keypad9,
        Key::KeypadDecimal,
        Key::KeypadDivide,
        Key::KeypadMultiply,
        Key::KeypadSubtract,
        Key::KeypadAdd,
        Key::KeypadEnter,
        Key::KeypadEqual,
        Key::AppBack,
        Key::AppForward,
        Key::Oem102,
        Key::GamepadStart,
        Key::GamepadBack,
        Key::GamepadFaceLeft,
        Key::GamepadFaceRight,
        Key::GamepadFaceUp,
        Key::GamepadFaceDown,
        Key::GamepadDpadLeft,
        Key::GamepadDpadRight,
        Key::GamepadDpadUp,
        Key::GamepadDpadDown,
        Key::GamepadL1,
        Key::GamepadR1,
        Key::GamepadL2,
        Key::GamepadR2,
        Key::GamepadL3,
        Key::GamepadR3,
        Key::GamepadLStickLeft,
        Key::GamepadLStickRight,
        Key::GamepadLStickUp,
        Key::GamepadLStickDown,
        Key::GamepadRStickLeft,
        Key::GamepadRStickRight,
        Key::GamepadRStickUp,
        Key::GamepadRStickDown,
        Key::MouseLeft,
        Key::MouseRight,
        Key::MouseMiddle,
        Key::MouseX1,
        Key::MouseX2,
        Key::MouseWheelX,
        Key::MouseWheelY,
        Key::ReservedForModCtrl,
        Key::ReservedForModShift,
        Key::ReservedForModAlt,
        Key::ReservedForModSuper,
        Key::ModCtrl,
        Key::ModShift,
        Key::ModAlt,
        Key::ModSuper,
    ];
    /// Total count of `Key` variants
    pub const COUNT: usize = sys::ImGuiKey_NamedKey_COUNT as usize + 4;
}

/// Target widget selection for keyboard focus
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum FocusedWidget {
    /// Previous widget
    Previous,
    /// Next widget
    Next,
    /// Widget using a relative positive offset (0 is the next widget).
    ///
    /// Use this to access sub components of a multiple component widget.
    Offset(u32),
}

impl FocusedWidget {
    #[inline]
    fn as_offset(self) -> i32 {
        match self {
            FocusedWidget::Previous => -1,
            FocusedWidget::Next => 0,
            FocusedWidget::Offset(offset) => offset as i32,
        }
    }
}

/// # Input: Keyboard
impl Ui {
    /// Returns true if the key is being held.
    #[inline]
    #[doc(alias = "IsKeyDown")]
    pub fn is_key_down(&self, key: Key) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(feature = "docking")] {
                unsafe { sys::igIsKeyDown_Nil(key as u32) }
            } else {
                unsafe { sys::igIsKeyDown(key as u32) }
            }
        }
    }

    /// Returns true if the key was pressed (went from !down to down).
    ///
    /// Affected by key repeat settings (`io.key_repeat_delay`, `io.key_repeat_rate`)
    #[inline]
    #[doc(alias = "IsKeyPressed")]
    pub fn is_key_pressed(&self, key: Key) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(feature = "docking")] {
                unsafe { sys::igIsKeyPressed_Bool(key as u32, true) }
            } else {
                unsafe { sys::igIsKeyPressed(key as u32, true) }
            }
        }
    }

    /// Returns true if the key was pressed (went from !down to down).
    ///
    /// Is **not** affected by key repeat settings (`io.key_repeat_delay`, `io.key_repeat_rate`)
    #[inline]
    #[doc(alias = "IsKeyPressed")]
    pub fn is_key_pressed_no_repeat(&self, key: Key) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(feature = "docking")] {
                unsafe { sys::igIsKeyPressed_Bool(key as u32, false) }
            } else {
                unsafe { sys::igIsKeyPressed(key as u32, false) }
            }
        }
    }

    /// Returns true if the key was released (went from down to !down)
    #[inline]
    #[doc(alias = "IsKeyReleased")]
    pub fn is_key_released(&self, key: Key) -> bool {
        cfg_if::cfg_if! {
            if #[cfg(feature = "docking")] {
                unsafe { sys::igIsKeyReleased_Nil(key as u32) }
            } else {
                unsafe { sys::igIsKeyReleased(key as u32) }
            }
        }
    }

    /// Returns a count of key presses using the given repeat rate/delay settings.
    ///
    /// Usually returns 0 or 1, but might be >1 if `rate` is small enough that `io.delta_time` >
    /// `rate`.
    #[inline]
    #[doc(alias = "GetKeyPressedAmount")]
    pub fn key_pressed_amount(&self, key: Key, repeat_delay: f32, rate: f32) -> u32 {
        unsafe { sys::igGetKeyPressedAmount(key as u32, repeat_delay, rate) as u32 }
    }

    /// Focuses keyboard on the next widget.
    ///
    /// This is the equivalent to [set_keyboard_focus_here_with_offset](Self::set_keyboard_focus_here_with_offset)
    /// with `target_widget` set to `FocusedWidget::Next`.
    #[inline]
    #[doc(alias = "SetKeyboardFocusHere")]
    pub fn set_keyboard_focus_here(&self) {
        self.set_keyboard_focus_here_with_offset(FocusedWidget::Next);
    }

    /// Focuses keyboard on a widget relative to current position.
    #[inline]
    #[doc(alias = "SetKeyboardFocusHere")]
    pub fn set_keyboard_focus_here_with_offset(&self, target_widget: FocusedWidget) {
        unsafe {
            sys::igSetKeyboardFocusHere(target_widget.as_offset());
        }
    }
}
