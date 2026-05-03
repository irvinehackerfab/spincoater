# Motor Controller: ESP32
This is a [Rust](https://rust-lang.org/) crate with binaries that you flash onto the [ESP32](https://www.espressif.com/en/products/socs/esp32) with [espflash](https://docs.espressif.com/projects/rust/book/getting-started/tooling/espflash.html).

The guide for Rust programming on ESP32 can be found [here](https://docs.espressif.com/projects/rust/book/preface.html). You can either follow this guide for setting up your dev environment, or you can use the one provided and documented in [DEVELOPMENT.md](DEVELOPMENT.md).

The crate was initially generated from [esp-generate](https://docs.espressif.com/projects/rust/book/getting-started/tooling/esp-generate.html).

# Configuring your editor
The `editor_configurations` folder contains default configurations for various editors. To avoid conflicting with any configurations you may have, they have no effect until you move them out into the `mc_esp32` folder and add a `.` to the front of the name.

# Programs
## __Note__
[0, RX, TX, EN, 12, 13, 14, 15 and 3V3](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block) may be used for the [ESP-PROG-2](https://docs.espressif.com/projects/esp-dev-kits/en/latest/other/esp-prog-2/user_guide.html#header-block) in the future.

## `pwm`
This is a basic program that initializes PWM on pin [IO26](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block) and sets it to a constant duty cycle of 5% with a frequency of 50hz.

This is useful if you ever need to do a simple check to make sure our current ESC isn't misbehaving.

Run with `cargo run --release --bin pwm`

## `spincoater`
This program does the following:
- Enables UART communication over the pins:
  - TX: IO23
  - RX: IO35
  - The ESP-Prog-2 can be used to connect a PC to this UART interface.
  - Programs must use [postcard-rpc](https://github.com/jamesmunns/postcard-rpc) and the protocol defined in `sc_messages` (in the workspace above this one) to successfully communicate with the MCU.
- Initializes PWM on pin [IO26](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block)
  - Starts with a constant duty cycle of 5% at a frequency of 50hz.
- Records hall effect sensor input on pin [IO27](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block)
- Initializes the [TFT display](https://protosupplies.com/product/tft-lcd-2-8-240x320-rgb-spi-display-with-touchscreen/) with the following pins:
  - MISO: GPIO19
  - MOSI: GPIO23
  - SCK: GPIO18
  - CS: GPIO15
  - DC: GPIO2
  - RESET: GPIO4
  - T_CS (touch chip select, permanently low for now): GPIO33
  - LED: GPIO22
  - 5V is used to power the LCD.
- Controls the vacuum pump on pin [IO17](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block).
  - Active high

The display is used to report errors with the UART communication.

Run with `cargo run --bin spincoater`.

If the program crashes with the error message `Detected a write to the stack guard value on AppCpu`, it means a stack overflowed. You'll likely need to increase the second core stack size in [`lib.rs`](src/lib.rs).
