use embassy_rp::pio::{
    self, Common, Config, FifoJoin, Instance, LoadedProgram, PioPin, ShiftConfig, ShiftDirection,
    StateMachine,
};
use embassy_rp::Peri;
use embassy_time::{Duration, Timer};
use fixed::traits::ToFixed;

// GameCube controller communication protocol using PIO
// This implements bidirectional communication with GameCube controllers

pub struct ControllerProgram<'d, PIO: Instance> {
    prg: LoadedProgram<'d, PIO>,
}

impl<'d, PIO: Instance> ControllerProgram<'d, PIO> {
    pub fn new(common: &mut Common<'d, PIO>) -> Self {
        // Build the PIO program using the Assembler API
        // Note: We use the Assembler instead of pio_asm!/pio_file! because:
        // 1. pio_asm! macro isn't readily available in pio 0.3 when used with embassy-rp
        // 2. The Assembler API is more explicit and type-safe
        // 3. It provides better error messages and IDE support
        let prg = build_program();
        Self {
            prg: common.load_program(&prg),
        }
    }
}

fn build_program() -> pio::program::Program<32> {
    const T1: u8 = 4;
    const T2: u8 = 12;

    let mut a =
        pio::program::Assembler::<32>::new_with_side_set(pio::program::SideSet::new(true, 1, false));

    let mut wrap_target = a.label();
    let mut wrap_source = a.label();
    let mut send_data = a.label();
    let mut do_zero = a.label();
    let mut do_one = a.label();
    let mut send_stop = a.label();
    let mut receive_byte = a.label();
    let mut get_bit = a.label();

    a.bind(&mut wrap_target);

    // out y, 8 side 0 ; load # of bytes expected in response
    a.out_with_side_set(pio::program::OutDestination::Y, 8, 0);

    // Delay for 32 cycles (4x nop [7])
    a.nop_with_delay(7);
    a.nop_with_delay(7);
    a.nop_with_delay(7);
    a.nop_with_delay(7);

    // sendData:
    a.bind(&mut send_data);
    // out x, 1 side 0 [T1 - 1]; send data bit by bit
    a.out_with_delay_and_side_set(pio::program::OutDestination::X, 1, T1 - 1, 0);

    // jmp !x do_zero side 1 [T1 - 1] ; set 0 for 4 cycles
    a.jmp_with_delay_and_side_set(pio::program::JmpCondition::XIsZero, &mut do_zero, T1 - 1, 1);

    // do_one:
    a.bind(&mut do_one);
    // jmp !OSRE sendData side 0 [T1 * 2 - 1] ; set 1 for 10 cycles
    a.jmp_with_delay_and_side_set(
        pio::program::JmpCondition::OutputShiftRegisterNotEmpty,
        &mut send_data,
        T1 * 2 - 1,
        0,
    );
    // jmp sendStop [T1 - 1]
    a.jmp_with_delay(pio::program::JmpCondition::Always, &mut send_stop, T1 - 1);

    // do_zero:
    a.bind(&mut do_zero);
    // jmp !OSRE sendData [T1 * 2 - 1]; set 1
    a.jmp_with_delay(
        pio::program::JmpCondition::OutputShiftRegisterNotEmpty,
        &mut send_data,
        T1 * 2 - 1,
    );
    // jmp sendStop side 0 [T1 - 1]
    a.jmp_with_delay_and_side_set(pio::program::JmpCondition::Always, &mut send_stop, T1 - 1, 0);

    // sendStop:
    a.bind(&mut send_stop);
    // nop side 1 [T1 - 1] ; send the stop bit (1)
    a.nop_with_delay_and_side_set(T1 - 1, 1);
    // nop side 0
    a.nop_with_side_set(0);

    // receiveByte:
    a.bind(&mut receive_byte);
    // set x, 7
    a.set(pio::program::SetDestination::X, 7);

    // getBit:
    a.bind(&mut get_bit);
    // wait 0 pin 0 [T1 + 1]; wait until the line goes low
    a.wait_with_delay(0, pio::program::WaitSource::PIN, 0, false, T1 + 1);
    // in pins 1 ; read
    a.r#in(pio::program::InSource::PINS, 1);
    // wait 1 pin 0 ; wait for a 1 if it's a 0
    a.wait(1, pio::program::WaitSource::PIN, 0, false);
    // jmp x-- getBit ; get the next bit
    a.jmp(pio::program::JmpCondition::XDecNonZero, &mut get_bit);
    // jmp y-- receiveByte ; get the next byte
    a.jmp(pio::program::JmpCondition::YDecNonZero, &mut receive_byte);

    a.bind(&mut wrap_source);

    a.assemble_with_wrap(wrap_source, wrap_target)
}

pub struct Controller<'d, PIO: Instance, const SM: usize> {
    sm: StateMachine<'d, PIO, SM>,
    controller_state: [u8; 8],
    rumble: bool,
}

impl<'d, PIO: Instance, const SM: usize> Controller<'d, PIO, SM> {
    pub fn new(
        common: &mut Common<'d, PIO>,
        mut sm: StateMachine<'d, PIO, SM>,
        pin: Peri<'d, impl PioPin>,
        _program: &ControllerProgram<'d, PIO>,
    ) -> Self {
        // Configure the state machine
        let mut cfg = Config::default();

        // Configure shift registers
        // Out shift: MSB first, autopull at 8 bits
        cfg.shift_out = ShiftConfig {
            auto_fill: true,
            direction: ShiftDirection::Left,
            threshold: 8,
        };

        // In shift: MSB first, autopush at 8 bits
        cfg.shift_in = ShiftConfig {
            auto_fill: true,
            direction: ShiftDirection::Left,
            threshold: 8,
        };

        // Configure the pin
        let pin = common.make_pio_pin(pin);
        cfg.set_in_pins(&[&pin]);
        cfg.set_out_pins(&[&pin]);
        cfg.set_set_pins(&[&pin]);
        cfg.use_program(&_program.prg, &[&pin]);

        // Calculate clock divider
        // The original uses: clock_get_hz(clk_sys) / (cyclesPerBit * frequency)
        // where T1 = 4, T2 = 12
        // cyclesPerBit = (T1 + T2) / 4 = (4 + 12) / 4 = 4
        // frequency = 1000000 (1 MHz)
        // For a 125 MHz system clock: 125000000 / (4 * 1000000) = 31.25
        const T1: u8 = 4;
        const T2: u8 = 12;
        let cycles_per_bit = (T1 + T2) / 4;
        let frequency = 1_000_000f32; // 1 MHz
        let system_clock = 125_000_000f32; // 125 MHz for RP2040
        let clock_div = system_clock / (f32::from(cycles_per_bit) * frequency);
        cfg.clock_divider = clock_div.to_fixed();

        // Don't join FIFOs - we need both TX and RX
        cfg.fifo_join = FifoJoin::Duplex;

        sm.set_config(&cfg);
        sm.set_enable(true);

        Self {
            sm,
            controller_state: [0u8; 8],
            rumble: false,
        }
    }

    pub fn set_rumble(&mut self, rumble: bool) {
        self.rumble = rumble;
    }

    pub fn get_controller_state(&self) -> &[u8; 8] {
        &self.controller_state
    }

    /// Initialize the controller
    pub async fn init(&mut self) {
        let request = [0x41u8];
        let mut response = [0u8; 3];
        self.transfer(&request, &mut response).await;
    }

    /// Update the controller state
    pub async fn update_state(&mut self) {
        let rumble_byte = u8::from(self.rumble);
        let request = [0x40u8, 0x03u8, rumble_byte];
        let mut temp_state = [0u8; 8];
        self.transfer(&request, &mut temp_state).await;
        self.controller_state = temp_state;
    }

    /// Transfer data to/from the controller
    async fn transfer(&mut self, request: &[u8], response: &mut [u8]) {
        // Clear FIFOs
        self.sm.clear_fifos();

        // Send the number of response bytes expected (responseLength - 1) in the upper 5 bits
        let response_count = ((response.len() - 1) & 0x1F) as u32;
        self.sm.tx().wait_push(response_count << 24).await;

        // Send request bytes
        for &byte in request {
            self.sm.tx().wait_push(u32::from(byte) << 24).await;
        }

        // Receive response bytes with timeout
        for i in 0..response.len() {
            // Try to read with a timeout (600 microseconds as in original)
            let timeout = Timer::after(Duration::from_micros(600));

            // Wait for data or timeout
            match embassy_futures::select::select(
                self.sm.rx().wait_pull(),
                timeout,
            )
            .await
            {
                embassy_futures::select::Either::First(data) => {
                    response[i] = (data & 0xFF) as u8;
                }
                embassy_futures::select::Either::Second(()) => {
                    // Timeout - leave rest of response as zeros
                    break;
                }
            }
        }

        // Delay as in original: 4 * (requestLength + responseLength) + 450 microseconds
        let delay_us = 4 * (request.len() + response.len()) + 450;
        Timer::after(Duration::from_micros(delay_us as u64)).await;
    }
}
