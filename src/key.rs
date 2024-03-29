#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum Key {
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
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
    Space,
    Minus,
    Equals,
    Grave,
    Tab,
    CapsLock,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Apostrophe,
    Comma,
    Period,
    Slash,
    Alt,
    Control,
    Shift,
    Meta,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Backspace,
    Escape,
    Enter,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
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
}

impl Key {
    pub fn from_char(char: u8) -> Option<Self> {
        let key = match char {
            b'0' => Self::Num0,
            b'1' => Self::Num1,
            b'2' => Self::Num2,
            b'3' => Self::Num3,
            b'4' => Self::Num4,
            b'5' => Self::Num5,
            b'6' => Self::Num6,
            b'7' => Self::Num7,
            b'8' => Self::Num8,
            b'9' => Self::Num9,
            b'a' => Self::A,
            b'b' => Self::B,
            b'c' => Self::C,
            b'd' => Self::D,
            b'e' => Self::E,
            b'f' => Self::F,
            b'g' => Self::G,
            b'h' => Self::H,
            b'i' => Self::I,
            b'j' => Self::J,
            b'k' => Self::K,
            b'l' => Self::L,
            b'm' => Self::M,
            b'n' => Self::N,
            b'o' => Self::O,
            b'p' => Self::P,
            b'q' => Self::Q,
            b'r' => Self::R,
            b's' => Self::S,
            b't' => Self::T,
            b'u' => Self::U,
            b'v' => Self::V,
            b'w' => Self::W,
            b'x' => Self::X,
            b'y' => Self::Y,
            b'z' => Self::Z,
            b'-' => Self::Minus,
            b'=' => Self::Equals,
            b'[' => Self::LeftBracket,
            b']' => Self::RightBracket,
            b'\\' => Self::Backslash,
            b';' => Self::Semicolon,
            b'\'' => Self::Apostrophe,
            b',' => Self::Comma,
            b'.' => Self::Period,
            b'/' => Self::Slash,
            _ => return None,
        };
        Some(key)
    }
    pub fn to_char(self) -> Option<u8> {
        let char = match self {
            Self::Num0 => b'0',
            Self::Num1 => b'1',
            Self::Num2 => b'2',
            Self::Num3 => b'3',
            Self::Num4 => b'4',
            Self::Num5 => b'5',
            Self::Num6 => b'6',
            Self::Num7 => b'7',
            Self::Num8 => b'8',
            Self::Num9 => b'9',
            Self::A => b'a',
            Self::B => b'b',
            Self::C => b'c',
            Self::D => b'd',
            Self::E => b'e',
            Self::F => b'f',
            Self::G => b'g',
            Self::H => b'h',
            Self::I => b'i',
            Self::J => b'j',
            Self::K => b'k',
            Self::L => b'l',
            Self::M => b'm',
            Self::N => b'n',
            Self::O => b'o',
            Self::P => b'p',
            Self::Q => b'q',
            Self::R => b'r',
            Self::S => b's',
            Self::T => b't',
            Self::U => b'u',
            Self::V => b'v',
            Self::W => b'w',
            Self::X => b'x',
            Self::Y => b'y',
            Self::Z => b'z',
            Self::Minus => b'-',
            Self::Equals => b'=',
            Self::LeftBracket => b'[',
            Self::RightBracket => b']',
            Self::Backslash => b'\\',
            Self::Semicolon => b';',
            Self::Apostrophe => b'\'',
            Self::Comma => b',',
            Self::Period => b'.',
            Self::Slash => b'/',
            _ => return None,
        };
        Some(char)
    }
}
