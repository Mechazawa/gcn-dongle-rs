# USB Serial Logging

The RP2040 now provides USB serial logging for real-time debugging without needing a debug probe.

## How It Works

The firmware creates a USB CDC-ACM (Communications Device Class - Abstract Control Model) serial port that appears as `/dev/tty.usbmodemGCN0011` (macOS) or `COMx` (Windows) or `/dev/ttyACMx` (Linux) when the device is connected.

## Viewing Logs

### macOS/Linux
```bash
# Find the device
ls /dev/tty.usbmodem*

# Connect with screen
screen /dev/tty.usbmodemGCN0011 115200

# Or use cu
cu -l /dev/tty.usbmodemGCN0011 -s 115200

# Or use cat (read-only)
cat /dev/tty.usbmodemGCN0011
```

To exit `screen`: Press `Ctrl-A` then `K` then `Y`

### Windows
Use PuTTY, TeraTerm, or Arduino Serial Monitor:
- Baud rate: Any (ignored by USB CDC)
- Port: Look for "GameCube Controller" in Device Manager

## Usage in Code

Two options for logging:

### 1. USB Serial Only (Current Implementation)
```rust
usb_log!("Controller ready!");
usb_log!("Button state: A={}", state.a());
```

### 2. Dual Logging (defmt + USB Serial)
Both debug probe (RTT) and USB serial get the logs:
```rust
info!("Message goes to RTT");  // defmt only
usb_log!("Message goes to USB serial only");
```

## Features

- **Non-blocking**: Logs won't stall your code if USB is disconnected
- **Automatic reconnection**: Reconnect anytime and logs will resume
- **Buffered**: Up to 16 messages queued (128 bytes each)
- **Welcome banner**: Shows connection status on connect

## Current Log Output

The firmware logs:
- System initialization
- Controller initialization status
- Button state changes (debounced)
- Analog stick values (every 1 second)

## Customization

Edit `src/main.rs` to change what gets logged:
```rust
// Add more USB logging
usb_log!("Custom message: {}", value);

// Change update frequency
Timer::after_millis(17).await;  // 60Hz
```

Edit `src/usb_logger.rs` to change buffer sizes:
```rust
// Increase message capacity or size
pub static USB_LOG_CHANNEL: Channel<..., heapless::String<256>, 32> = ...;
```
