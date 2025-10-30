#![no_std]
#![no_main]

mod controller;
mod controller_state;
mod usb_logger;

use defmt_rtt as _;
use panic_probe as _;

use controller::{Controller, ControllerProgram};
use controller_state::ControllerState;
use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::config::Config;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio;
use embassy_time::Timer;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Config::default());

    info!("GameCube Controller Dongle starting...");

    // Initialize USB serial logging
    let (usb_device, usb_class) = usb_logger::init_usb(p.USB);

    // Spawn USB tasks
    spawner.spawn(usb_logger::usb_task(usb_device).unwrap());
    spawner.spawn(usb_logger::usb_logger_task(usb_class).unwrap());

    usb_log!("=== System Initialized ===");

    // Initialize PIO for controller communication
    let pio::Pio {
        mut common,
        sm0,
        ..
    } = pio::Pio::new(p.PIO0, Irqs);

    // Load the controller program
    let program = ControllerProgram::new(&mut common);

    // Create controller on pin 10 with state machine 0
    let mut controller = Controller::new(&mut common, sm0, p.PIN_10, &program);

    // Initialize the controller
    info!("Initializing controller...");
    usb_log!("Initializing GameCube controller on PIN_10...");
    controller.init().await;

    info!("Controller initialized!");
    usb_log!("Controller ready!");

    // Main loop - poll controller state
    let mut last_buttons = 0u16;
    loop {
        controller.update_state().await;

        // Create ControllerState view of the raw state (matches C++ pattern)
        let state = ControllerState::new(controller.get_controller_state());

        // Build current button state for change detection
        let current_buttons = (u16::from(state.a()) << 0)
            | (u16::from(state.b()) << 1)
            | (u16::from(state.x()) << 2)
            | (u16::from(state.y()) << 3)
            | (u16::from(state.start()) << 4)
            | (u16::from(state.z()) << 5)
            | (u16::from(state.l()) << 6)
            | (u16::from(state.r()) << 7);

        // Log button state changes to USB (reduces spam)
        if current_buttons != last_buttons {
            usb_log!(
                "Buttons: A={} B={} X={} Y={} Start={} Z={} L={} R={}",
                u8::from(state.a()),
                u8::from(state.b()),
                u8::from(state.x()),
                u8::from(state.y()),
                u8::from(state.start()),
                u8::from(state.z()),
                u8::from(state.l()),
                u8::from(state.r())
            );
            last_buttons = current_buttons;
        }

        // Log analog values periodically (every 60 polls = ~1 second at 60Hz)
        static mut POLL_COUNT: u32 = 0;
        unsafe {
            POLL_COUNT += 1;
            if POLL_COUNT % 60 == 0 {
                usb_log!(
                    "Analog - Stick:({},{}) C-Stick:({},{}) L:{} R:{}",
                    state.ax(),
                    state.ay(),
                    state.cx(),
                    state.cy(),
                    state.al(),
                    state.ar()
                );
            }
        }

        // Wait a bit before next poll (60Hz = ~16.67ms)
        Timer::after_millis(17).await;
    }
}