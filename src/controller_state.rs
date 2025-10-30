// GameCube controller state parser
// Matches the C++ ControllerState implementation exactly

// GC Joystick ranges
pub const GC_JOYSTICK_MIN: u8 = 0x00;
pub const GC_JOYSTICK_MID: u8 = 0x80;
pub const GC_JOYSTICK_MAX: u8 = 0xFF;

// GC First Byte masks
const GC_MASK_A: u8 = 0x1;
const GC_MASK_B: u8 = 0x1 << 1;
const GC_MASK_X: u8 = 0x1 << 2;
const GC_MASK_Y: u8 = 0x1 << 3;
const GC_MASK_START: u8 = 0x1 << 4;

// GC Second Byte masks
const GC_MASK_DPAD: u8 = 0xF;
const GC_MASK_Z: u8 = 0x1 << 4;
const GC_MASK_R: u8 = 0x1 << 5;
const GC_MASK_L: u8 = 0x1 << 6;

// D-pad directional values
const GC_MASK_DPAD_UP: u8 = 0x8;
const GC_MASK_DPAD_UPRIGHT: u8 = 0xA;
const GC_MASK_DPAD_RIGHT: u8 = 0x2;
const GC_MASK_DPAD_DOWNRIGHT: u8 = 0x6;
const GC_MASK_DPAD_DOWN: u8 = 0x4;
const GC_MASK_DPAD_DOWNLEFT: u8 = 0x5;
const GC_MASK_DPAD_LEFT: u8 = 0x1;
const GC_MASK_DPAD_UPLEFT: u8 = 0x9;

/// D-pad hat position for USB HID reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum HatPosition {
    Idle = 0,
    Up = 1,
    UpRight = 2,
    Right = 3,
    DownRight = 4,
    Down = 5,
    DownLeft = 6,
    Left = 7,
    UpLeft = 8,
}

/// GameCube controller state
/// Holds a reference to the raw 8-byte state buffer
#[derive(Debug, Clone, Copy)]
pub struct ControllerState<'a> {
    state: &'a [u8; 8],
}

impl<'a> ControllerState<'a> {
    /// Create a new ControllerState from raw state buffer
    pub fn new(state: &'a [u8; 8]) -> Self {
        Self { state }
    }

    // Button getters
    pub fn start(&self) -> bool {
        (self.state[0] & GC_MASK_START) != 0
    }

    pub fn a(&self) -> bool {
        (self.state[0] & GC_MASK_A) != 0
    }

    pub fn b(&self) -> bool {
        (self.state[0] & GC_MASK_B) != 0
    }

    pub fn x(&self) -> bool {
        (self.state[0] & GC_MASK_X) != 0
    }

    pub fn y(&self) -> bool {
        (self.state[0] & GC_MASK_Y) != 0
    }

    pub fn l(&self) -> bool {
        (self.state[1] & GC_MASK_L) != 0
    }

    pub fn r(&self) -> bool {
        (self.state[1] & GC_MASK_R) != 0
    }

    pub fn z(&self) -> bool {
        (self.state[1] & GC_MASK_Z) != 0
    }

    // D-pad getters
    pub fn dpad_up(&self) -> bool {
        (self.state[1] & GC_MASK_DPAD & GC_MASK_DPAD_UP) != 0
    }

    pub fn dpad_right(&self) -> bool {
        (self.state[1] & GC_MASK_DPAD & GC_MASK_DPAD_RIGHT) != 0
    }

    pub fn dpad_down(&self) -> bool {
        (self.state[1] & GC_MASK_DPAD & GC_MASK_DPAD_DOWN) != 0
    }

    pub fn dpad_left(&self) -> bool {
        (self.state[1] & GC_MASK_DPAD & GC_MASK_DPAD_LEFT) != 0
    }

    /// Get the d-pad as a hat position for USB HID
    pub fn dpad(&self) -> HatPosition {
        match self.state[1] & GC_MASK_DPAD {
            GC_MASK_DPAD_UP => HatPosition::Up,
            GC_MASK_DPAD_UPRIGHT => HatPosition::UpRight,
            GC_MASK_DPAD_RIGHT => HatPosition::Right,
            GC_MASK_DPAD_DOWNRIGHT => HatPosition::DownRight,
            GC_MASK_DPAD_DOWN => HatPosition::Down,
            GC_MASK_DPAD_DOWNLEFT => HatPosition::DownLeft,
            GC_MASK_DPAD_LEFT => HatPosition::Left,
            GC_MASK_DPAD_UPLEFT => HatPosition::UpLeft,
            _ => HatPosition::Idle,
        }
    }

    // Analog stick getters
    pub fn ax(&self) -> u8 {
        self.state[2]
    }

    pub fn ay(&self) -> u8 {
        self.state[3]
    }

    pub fn cx(&self) -> u8 {
        self.state[4]
    }

    pub fn cy(&self) -> u8 {
        self.state[5]
    }

    pub fn al(&self) -> u8 {
        self.state[6]
    }

    pub fn ar(&self) -> u8 {
        self.state[7]
    }
}
