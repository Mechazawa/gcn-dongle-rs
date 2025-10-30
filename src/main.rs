#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::config::Config;
use embassy_time::Timer;
use ssd1306::prelude::*;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let _ = embassy_rp::init(Config::default());

    // Main task just keeps the executor alive
    loop {
        Timer::after_secs(60).await;
    }
}
