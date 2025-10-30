// GameCube controller state parser
// This module parses the raw 8-byte controller state into individual fields

#[derive(Debug, Clone, Copy, Default)]
pub struct ControllerState {
    // Buttons
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub start: bool,
    pub z: bool,
    pub l: bool,
    pub r: bool,
    pub d_up: bool,
    pub d_down: bool,
    pub d_left: bool,
    pub d_right: bool,

    // Analog sticks (0-255, center is ~128)
    pub stick_x: u8,
    pub stick_y: u8,
    pub c_stick_x: u8,
    pub c_stick_y: u8,

    // Analog triggers (0-255)
    pub l_analog: u8,
    pub r_analog: u8,
}

impl ControllerState {
    /// Parse the raw 8-byte controller state
    pub fn from_raw(raw: &[u8; 8]) -> Self {
        // Byte 0: Buttons (high bits)
        // Bit 7: 0 (unused)
        // Bit 6: 0 (unused)
        // Bit 5: 0 (unused)
        // Bit 4: Start
        // Bit 3: Y
        // Bit 2: X
        // Bit 1: B
        // Bit 0: A

        // Byte 1: Buttons (low bits)
        // Bit 7: 1 (unused)
        // Bit 6: L
        // Bit 5: R
        // Bit 4: Z
        // Bit 3: D-pad Up
        // Bit 2: D-pad Down
        // Bit 1: D-pad Right
        // Bit 0: D-pad Left

        Self {
            // Byte 0 buttons
            start: (raw[0] & 0x10) != 0,
            y: (raw[0] & 0x08) != 0,
            x: (raw[0] & 0x04) != 0,
            b: (raw[0] & 0x02) != 0,
            a: (raw[0] & 0x01) != 0,

            // Byte 1 buttons
            l: (raw[1] & 0x40) != 0,
            r: (raw[1] & 0x20) != 0,
            z: (raw[1] & 0x10) != 0,
            d_up: (raw[1] & 0x08) != 0,
            d_down: (raw[1] & 0x04) != 0,
            d_right: (raw[1] & 0x02) != 0,
            d_left: (raw[1] & 0x01) != 0,

            // Byte 2-3: Left stick
            stick_x: raw[2],
            stick_y: raw[3],

            // Byte 4-5: C-stick
            c_stick_x: raw[4],
            c_stick_y: raw[5],

            // Byte 6-7: Analog triggers
            l_analog: raw[6],
            r_analog: raw[7],
        }
    }

    /// Update the state from new raw data
    pub fn update(&mut self, raw: &[u8; 8]) {
        *self = Self::from_raw(raw);
    }
}
