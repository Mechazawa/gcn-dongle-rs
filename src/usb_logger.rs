// USB Serial Logger
// Provides a USB CDC-ACM serial port for logging and debugging

use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config, UsbDevice};
use static_cell::StaticCell;

pub type UsbDriver = Driver<'static, USB>;

/// USB logger message channel (capacity 16 messages, 128 bytes each)
pub static USB_LOG_CHANNEL: Channel<CriticalSectionRawMutex, heapless::String<128>, 16> =
    Channel::new();

bind_interrupts!(pub struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

/// Initialize USB serial device
pub fn init_usb(
    usb_peripheral: Peri<'static, USB>,
) -> (UsbDevice<'static, UsbDriver>, CdcAcmClass<'static, UsbDriver>) {
    // Create USB driver
    let driver = Driver::new(usb_peripheral, Irqs);

    // Configure USB device
    let mut config = Config::new(0x16c0, 0x27dd); // USB Test VID/PID
    config.manufacturer = Some("GCN Adapter");
    config.product = Some("GameCube Controller");
    config.serial_number = Some("GCN001");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Allocate static buffers for USB
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [],
        CONTROL_BUF.init([0; 64]),
    );

    // Create CDC-ACM class (USB serial)
    static STATE: StaticCell<State> = StaticCell::new();
    let state = STATE.init(State::new());
    let class = CdcAcmClass::new(&mut builder, state, 64);

    let usb_device = builder.build();

    (usb_device, class)
}

/// USB device task - must be spawned
#[embassy_executor::task]
pub async fn usb_task(mut usb: UsbDevice<'static, UsbDriver>) -> ! {
    usb.run().await
}

/// USB logger task - reads from channel and writes to USB serial
#[embassy_executor::task]
pub async fn usb_logger_task(mut class: CdcAcmClass<'static, UsbDriver>) -> ! {
    loop {
        // Wait for USB connection
        class.wait_connection().await;

        // Send welcome message
        let _ = class
            .write_packet(b"\r\n=== GameCube Controller Dongle ===\r\n")
            .await;
        let _ = class
            .write_packet(b"USB Serial Logger Active\r\n\r\n")
            .await;

        // Process log messages until disconnected
        loop {
            let msg = USB_LOG_CHANNEL.receive().await;

            // Try to send message, break on disconnect
            if class.write_packet(msg.as_bytes()).await.is_err() {
                break;
            }
            if class.write_packet(b"\r\n").await.is_err() {
                break;
            }
        }
    }
}

/// Helper function to log a message to USB serial (non-blocking)
pub fn usb_log(msg: &str) {
    if let Ok(s) = heapless::String::<128>::try_from(msg) {
        let _ = USB_LOG_CHANNEL.try_send(s);
    }
}

/// Helper function to log a formatted message to USB serial (non-blocking)
#[macro_export]
macro_rules! usb_log {
    ($($arg:tt)*) => {{
        let mut s: heapless::String<128> = heapless::String::new();
        use core::fmt::Write;
        let _ = write!(&mut s, $($arg)*);
        let _ = $crate::usb_logger::USB_LOG_CHANNEL.try_send(s);
    }};
}
