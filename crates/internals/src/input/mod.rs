use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

#[derive(Debug, Clone, Copy)]
pub enum Binding {
    Key(Scancode),
    MouseButton(MouseButton),
    MouseCursor { x: f32, y: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyState {
    Down,
    Held,
    Up,
}

#[derive(Debug)]
pub struct Inputstate {
    keys_down: HashSet<Scancode>, // Only this frame
    keys_held: HashSet<Scancode>, // Only this frame
    keys_up: HashSet<Scancode>,   // Only this frame
    mouse_down: HashSet<MouseButton>,
    mouse_held: HashSet<MouseButton>,
    mouse_up: HashSet<MouseButton>,
    mouse_pos: (f32, f32),
    mouse_delta: (f32, f32),
}

impl Inputstate {
    pub fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            keys_held: HashSet::new(),
            keys_up: HashSet::new(),
            mouse_down: HashSet::new(),
            mouse_held: HashSet::new(),
            mouse_up: HashSet::new(),
            mouse_pos: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
        }
    }

    pub fn clear(&mut self) {
        self.keys_down.clear();
        self.keys_up.clear();
        self.mouse_down.clear();
        self.mouse_up.clear();
        self.mouse_delta = (0.0, 0.0);
    }

    pub fn key_down(&mut self, key: Scancode) {
        if self.keys_held.insert(key) {
            self.keys_down.insert(key);
        }
    }

    pub fn key_up(&mut self, key: Scancode) {
        if self.keys_held.remove(&key) {
            self.keys_up.insert(key);
        }
    }

    pub fn mouse_down(&mut self, button: MouseButton) {
        if self.mouse_held.insert(button) {
            self.mouse_down.insert(button);
        }
    }

    pub fn mouse_up(&mut self, button: MouseButton) {
        if self.mouse_held.remove(&button) {
            self.mouse_up.insert(button);
        }
    }

    pub fn mouse_motion(&mut self, x: f32, y: f32, x_rel: f32, y_rel: f32) {
        self.mouse_delta = (x_rel, y_rel);
        self.mouse_pos = (x, y);
    }

    pub fn key_state(&self, key: &Scancode) -> KeyState {
        if self.keys_down.contains(key) {
            KeyState::Down
        } else if self.keys_held.contains(key) {
            KeyState::Held
        } else {
            KeyState::Up
        }
    }

    pub fn mouse_button_state(&self, button: &MouseButton) -> KeyState {
        if self.mouse_down.contains(button) {
            KeyState::Down
        } else if self.mouse_held.contains(button) {
            KeyState::Held
        } else {
            KeyState::Up
        }
    }
}

pub struct InputMap<A: Hash + Eq + Copy> {
    map: HashMap<A, Binding>,
}

impl<A> InputMap<A>
where
    A: Hash + Eq + Copy,
{
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn bind(&mut self, action: A, binding: Binding) {
        self.map.insert(action, binding);
    }

    pub fn unbind(&mut self, action: A) {
        self.map.remove(&action);
    }

    pub fn get_all_bindings(&self) -> Vec<&Binding> {
        self.map.values().collect()
    }

    pub fn is_down(&self, binding: &A, state: &Inputstate) -> bool {
        self.map.get(binding).map_or(false, |b| match b {
            Binding::Key(scancode) => state.key_state(scancode) == KeyState::Down,
            Binding::MouseButton(mouse_button) => {
                state.mouse_button_state(mouse_button) == KeyState::Down
            }
            _ => false,
        })
    }

    pub fn is_held(&self, binding: &A, state: &Inputstate) -> bool {
        self.map.get(binding).map_or(false, |b| match b {
            Binding::Key(scancode) => state.key_state(scancode) == KeyState::Held,
            Binding::MouseButton(mouse_button) => {
                state.mouse_button_state(mouse_button) == KeyState::Held
            }
            _ => false,
        })
    }

    pub fn is_up(&self, binding: &A, state: &Inputstate) -> bool {
        self.map.get(binding).map_or(false, |b| match b {
            Binding::Key(scancode) => state.key_state(scancode) == KeyState::Up,
            Binding::MouseButton(mouse_button) => {
                state.mouse_button_state(mouse_button) == KeyState::Up
            }
            _ => false,
        })
    }
}

// Lifted from the SDL3 crate
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum MouseButton {
    Unknown,
    Left,
    Middle,
    Right,
    X1,
    X2,
}

// Lifted from the SDL3 crate
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Scancode {
    Unknown,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    _1,
    _2,
    _3,
    _4,
    _5,
    _6,
    _7,
    _8,
    _9,
    _0,
    Return,
    Escape,
    Backspace,
    Tab,
    Space,
    Minus,
    Equals,
    LeftBracket,
    RightBracket,
    Backslash,
    NonUsHash,
    Semicolon,
    Apostrophe,
    Grave,
    Comma,
    Period,
    Slash,
    CapsLock,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    PrintScreen,
    ScrollLock,
    Pause,
    Insert,
    Home,
    PageUp,
    Delete,
    End,
    PageDown,
    Right,
    Left,
    Down,
    Up,
    NumLockClear,
    KpDivide,
    KpMultiply,
    KpMinus,
    KpPlus,
    KpEnter,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    Kp0,
    KpPeriod,
    NonUsBackslash,
    Application,
    Power,
    KpEquals,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Execute,
    Help,
    Menu,
    Select,
    Stop,
    Again,
    Undo,
    Cut,
    Copy,
    Paste,
    Find,
    Mute,
    VolumeUp,
    VolumeDown,
    KpComma,
    KpEqualsAs400,
    International1,
    International2,
    International3,
    International4,
    International5,
    International6,
    International7,
    International8,
    International9,
    Lang1,
    Lang2,
    Lang3,
    Lang4,
    Lang5,
    Lang6,
    Lang7,
    Lang8,
    Lang9,
    AltErase,
    SysReq,
    Cancel,
    Clear,
    Prior,
    Return2,
    Separator,
    Out,
    Oper,
    ClearAgain,
    CrSel,
    ExSel,
    Kp00,
    Kp000,
    ThousandsSeparator,
    DecimalSeparator,
    CurrencyUnit,
    CurrencySubunit,
    KpLeftParen,
    KpRightParen,
    KpLeftBrace,
    KpRightBrace,
    KpTab,
    KpBackspace,
    KpA,
    KpB,
    KpC,
    KpD,
    KpE,
    KpF,
    KpXor,
    KpPower,
    KpPercent,
    KpLess,
    KpGreater,
    KpAmpersand,
    KpDblAmpersand,
    KpVerticalBar,
    KpDblVerticalBar,
    KpColon,
    KpHash,
    KpSpace,
    KpAt,
    KpExclam,
    KpMemStore,
    KpMemRecall,
    KpMemClear,
    KpMemAdd,
    KpMemSubtract,
    KpMemMultiply,
    KpMemDivide,
    KpPlusMinus,
    KpClear,
    KpClearEntry,
    KpBinary,
    KpOctal,
    KpDecimal,
    KpHexadecimal,
    LCtrl,
    LShift,
    LAlt,
    LGui,
    RCtrl,
    RShift,
    RAlt,
    RGui,
    Mode,
    Sleep,
    Wake,
    ChannelIncrement,
    ChannelDecrement,
    MediaPlay,
    MediaPause,
    MediaRecord,
    MediaFastForward,
    MediaRewind,
    MediaNextTrack,
    MediaPreviousTrack,
    MediaStop,
    MediaEject,
    MediaPlayPause,
    MediaSelect,
    AcNew,
    AcOpen,
    AcClose,
    AcExit,
    AcSave,
    AcPrint,
    AcProperties,
    AcSearch,
    AcHome,
    AcBack,
    AcForward,
    AcStop,
    AcRefresh,
    AcBookmarks,
    SoftLeft,
    SoftRight,
    Call,
    EndCall,
    Reserved,
    Count,
}
