#![no_std]
#![no_main]

mod controller;
mod controller_state;

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
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Config::default());

    info!("GameCube Controller Dongle starting...");

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
    controller.init().await;

    info!("Controller initialized!");

    // Main loop - poll controller state
    loop {
        controller.update_state().await;
        let state = ControllerState::from_raw(controller.get_controller_state());

        // Log button presses
        if state.a {
            info!("A button pressed!");
        }
        if state.start {
            info!("Start button pressed!");
        }

        // Log analog stick positions
        info!(
            "Stick: ({}, {}), C-Stick: ({}, {})",
            state.stick_x,
            state.stick_y,
            state.c_stick_x,
            state.c_stick_y
        );

        // Wait a bit before next poll (60Hz = ~16.67ms)
        Timer::after_millis(17).await;
    }
}
