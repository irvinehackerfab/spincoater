# Motor Controller: ESP32
This is a [Rust](https://rust-lang.org/) crate with binaries that you flash onto the [ESP32](https://www.espressif.com/en/products/socs/esp32) with [espflash](https://docs.espressif.com/projects/rust/book/getting-started/tooling/espflash.html).

The guide for Rust programming on ESP32 can be found [here](https://docs.espressif.com/projects/rust/book/preface.html). You can either follow this guide for setting up your dev environment, or you can use the one provided by and documented in [DEVELOPMENT.md](DEVELOPMENT.md).

The crate was initially generated from [esp-generate](https://docs.espressif.com/projects/rust/book/getting-started/tooling/esp-generate.html).

# Configuring your editor
The `editor_configurations` folder contains default configurations for various editors. To avoid conflicting with any configurations you may have, they have no effect until you move them out into the `mc_esp32` folder and add a `.` to the front of the name.

# Pin Layout
Pins **[6, 7, 8, 9, 10, 11](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block)** are shared with the flash memory and should not be used as normal GPIOs.

Pins **[0, RX, TX, EN, 12, 13, 14, 15 and 5V](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block)** need to be for the [ESP-PROG-2](https://docs.espressif.com/projects/esp-dev-kits/en/latest/other/esp-prog-2/user_guide.html#header-block).

## Unused Pins
The display is not used in this program. If we ever use it, the [display's pins](https://protosupplies.com/wp-content/uploads/2020/07/TFT-LCD-28-240x320-RGB-ILI9341-with-Touchscreen-Connections-Top-Side.jpg) will be connected as follows:
- Vcc: Powered by power PCB. Not connected to DevKitC.
- GND: Grounded by power PCB. Not connected to DevKitC.
- CS (Display Chip Select): **19**
- RESET: **18**
- DC (Data/Command): **25**
- MOSI and T_MOSI (Master Out Slave In): **33**
- SCK and T_CLK (Clock): **32**
- LED: Held at a constant 3.3V by the power PCB because the display should always be on.
- MISO and T_MISO (Master In Slave Out): **35**
- T_CS (Touch Chip Select): **16**
- T_IRQ (Touch Interrupt Request): **34**

# Programs
## `pwm`
This is a basic program that initializes PWM on pin **26** and sets it to a constant duty cycle of 5% with a frequency of 50hz.

This is useful if you ever need to do a simple check to make sure our current ESC isn't misbehaving.

Run with `cargo run --release --bin pwm`

## `spincoater`
This program does the following:
- Enables UART communication over the pins:
  - TX: **1 (TX)**
  - RX: **3 (RX)**
  - Programs must use [postcard-rpc](https://github.com/jamesmunns/postcard-rpc) and the protocol defined in `sc_messages` (in the workspace above this one) to successfully communicate with the MCU.
- Initializes PWM on pin **26**
  - Starts with a constant duty cycle of 5% at a frequency of 50hz.
- Records hall effect sensor input on pin **27**
- Controls the vacuum pump on pin **17**
  - Active high

Run with `cargo run --bin spincoater`.

### UART Communication over an Adapter
Alternatively, you can perform UART communication using pins other than TX and RX. This would allow you to keep `espflash`'s RTT monitor open while running the program. However, it requires a separate UART-to-USB adapter, such as the [ESP-Prog-2](https://docs.espressif.com/projects/esp-dev-kits/en/latest/other/esp-prog-2/user_guide.html#).

If you'd like to do this, run the program with `cargo run --bin spincoater -F uart_over_adapter`.
