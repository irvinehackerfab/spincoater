# Motor Controller: ESP32
This is a [Rust](https://rust-lang.org/) crate with binaries that you flash onto the [ESP32](https://www.espressif.com/en/products/socs/esp32) with [espflash](https://docs.espressif.com/projects/rust/book/getting-started/tooling/espflash.html).

The guide for Rust programming on ESP32 can be found [here](https://docs.espressif.com/projects/rust/book/preface.html).

This was generated from [esp-generate](https://docs.espressif.com/projects/rust/book/getting-started/tooling/esp-generate.html).

# Configuring your editor
The `editor_configurations` folder contains default configurations for various editors. To avoid conflicting with any configurations you may have, they have no effect until you move them out into the `mc_esp32` folder and add a `.` to the front of the name.

# Programs
## `pwm`
This is a basic program that initializes PWM on pin [IO12](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block) and sets it to a constant duty cycle of 5% with a frequency of 50hz.

Run with `cargo run --release --bin pwm`

## `spincoater`
This program does the following:
- Initializes PWM on pin [IO12](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block)
  - Outputs a constant duty cycle of 5% with a frequency of 50hz.
- Records hall effect sensor input on pin [IO16](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block)
  - Prints the plate revolutions per minute every second.
- Enables a Wifi access point
  - Allows one device to connect at a time
  - Listens on a TCP socket on port 8080
  - Send and receives messages defined in `sc_messages` (in the workspace above this one).
- Initializes the [TFT display](https://protosupplies.com/product/tft-lcd-2-8-240x320-rgb-spi-display-with-touchscreen/) with the following pins:
  - MISO: GPIO19
  - MOSI: GPIO23
  - SCK: GPIO18
  - CS: GPIO15
  - DC: GPIO2
  - RESET: GPIO4
  - T_CS (touch chip select, not used yet): GPIO33

When flashing the program, you must specify the Wifi's SSID and password through environment variables. One way to do this is by running the program with `SSID=_ PASSWORD=_ cargo run --release --bin spincoater`.

The password must be 8-64 characters or else the radio will panic during initialization.

You must set a static IP to connect to the wifi. For example:
- IP: 192.168.2.2
- Netmask: 255.255.255.0
- Gateway: 192.168.2.1

If the program crashes with the error message `Detected a write to the stack guard value on AppCpu`, it means a stack overflowed. You'll likely need to increase the second core stack size in [`lib.rs`](src/lib.rs).
