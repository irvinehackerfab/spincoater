# Motor Controller: ESP32
This is a [Rust](https://rust-lang.org/) workspace with programs that you flash onto the [ESP32](https://www.espressif.com/en/products/socs/esp32) with [espflash](https://docs.espressif.com/projects/rust/book/getting-started/tooling/espflash.html).

The guide for Rust programming on ESP32 can be found [here](https://docs.espressif.com/projects/rust/book/preface.html).

This was generated from [esp-generate](https://docs.espressif.com/projects/rust/book/getting-started/tooling/esp-generate.html).

The `editor_configurations` folder contains default configurations for various editors. To avoid conflicting with any configurations you may have, they have no effect until you move them out into the `esp32` folder and add a `.` to the front of the name.

# Programs
## `pwm`
This is a basic program that initializes PWM on pin [IO12](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block) and sets it to a constant duty cycle of 5% with a frequency of 50hz.

Run with `cargo run --release --bin pwm`

## `spincoater`
This program does the following:
- Initializes PWM on pin [IO12](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block)
  - Outputs a constant duty cycle of 5% with a frequency of 50hz.
- Records hall effect sensor input on pin [IO4](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block)
  - Prints the plate revolutions per minute every second.
- Enables a Wifi access point
  - Allows one device to connect at a time
  - Listens on a TCP socket on port 8080
  - Send and receives messages defined in `sc_messages` (in the workspace above this one).

When flashing the program, you must specify the Wifi's SSID and password through environment variables. One way to do this is by running the program with `SSID=_ PASSWORD=_ cargo run --release --bin spincoater`.

The password must be 8-64 characters or else the radio will panic during initialization.
