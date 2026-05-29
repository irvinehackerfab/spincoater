# ESP32
This is a [Rust](https://rust-lang.org/) crate with binaries that you flash onto the [ESP32](https://www.espressif.com/en/products/socs/esp32) with [espflash](https://docs.espressif.com/projects/rust/book/getting-started/tooling/espflash.html). We currently use an [ESP32-DevKitC V4](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html) to simplify development.

The guide for Rust programming on ESP32 can be found [here](https://docs.espressif.com/projects/rust/book/preface.html). You can either follow this guide for setting up your dev environment, or you can use the one provided by and documented in [DEVELOPMENT.md](DEVELOPMENT.md).

The crate was initially generated from [esp-generate](https://docs.espressif.com/projects/rust/book/getting-started/tooling/esp-generate.html).

# Configuring your editor
The `editor_configurations` folder contains default configurations for various editors. To avoid conflicting with any configurations you may have, they have no effect until you move them out into the [esp32](./) folder and add a `.` to the front of the name.

# Pin Layout
All of the ESP32 DevKitC's pins can be found [here](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32/esp32-devkitc/user_guide.html#header-block).

Pins **6, 7, 8, 9, 10, 11** are shared with the flash memory and should not be used as normal GPIOs.

Pins **12, 13, 14, and 15** must be kept available for debugging with the [ESP-PROG-2](https://docs.espressif.com/projects/esp-dev-kits/en/latest/other/esp-prog-2/user_guide.html#header-block).

Pins **0, RX, TX, 22, 23, and EN** must be kept available for flashing and UART communication with the host PC.

## Debugging and Communication Pins
The ESP-Prog-2 can be used as both a JTAG and UART adapter. The pins on the ESP-Prog-2 are connected to the DevKitC as follows:
- UART:
  - ESP_EN: Not connected to DevKitC. We can use a pin socket to connect this if we ever need it.
  - VDD: Not connected to anything (ESP-Prog-2 is powered through USB-C)
  - ESP_TXD: **23**
  - GND: Grounded by power PCB.
  - ESP_RXD: **22**
  - ESP_IO0: Not connected to DevKitC. We can use a pin socket to connect this if we ever need it.
- JTAG:
  - VDD: Not connected to anything (ESP-Prog-2 is powered through USB-C)
  - ESP_TMS: **14**
  - Every GND: Grounded by power PCB.
  - ESP_TCK: **13**
  - ESP_TDO: ~~**15**~~ See [ESC Workaround](#ESC-Workaround) at the bottom.
  - ESP_TDI: **12**
  - NC: Not connected to anything

# Binaries
## `pin_usage_checker`
This is not a program. Rather, it contains a function that declares variables for every pin we might use.

This should be kept up to date with our pin layout, as Rust's borrow checker will prevent us from accidentally using a pin for two different functions.

If your editor's rust-analyzer is functioning properly, it will notify you if you create pin conflicts.

## `pwm`
This is a basic program that initializes PWM on pin **26** and sets it to a constant duty cycle of 7.5% with a frequency of 50hz.

This is useful if you ever need to do a simple check to make sure our current ESC isn't misbehaving.

Run with `cargo run --release --bin pwm`

## `spincoater`
This program lets you run the spincoater at a single plate RPM value for a single time in seconds using the touchscreen. It does the following:
- Initializes PWM on pin **26**
  - Starts with a constant duty cycle of 7.5% at a frequency of 50hz.
- Records hall effect sensor input on pin **27**
- Controls the vacuum pump on pin **17**
  - Active high
- Initializes the display with the [display's pins](https://protosupplies.com/wp-content/uploads/2020/07/TFT-LCD-28-240x320-RGB-ILI9341-with-Touchscreen-Connections-Top-Side.jpg) connected as follows:
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

Run with `cargo run --bin spincoater`.

## `spincoater_with_pc`
This program lets you run the spincoater by sending commands to it from a PC. It does the following:
- Enables UART communication over the pins:
  - TX: **1 (TX)**
  - RX: **3 (RX)**
  - Programs must use [postcard-rpc](https://github.com/jamesmunns/postcard-rpc) and the protocol defined in `sc_messages` (in the workspace above this one) to successfully communicate with the MCU.
- Initializes PWM on pin **26**
  - Starts with a constant duty cycle of 7.5% at a frequency of 50hz.
- Records hall effect sensor input on pin **27**
- Controls the vacuum pump on pin **17**
  - Active high

Run with `cargo run --bin spincoater_with_pc`.

### UART Communication over an Adapter
You can perform UART communication using pins other than TX and RX. This would allow you to keep `espflash`'s RTT monitor open while running the program. However, it requires a separate UART-to-USB adapter, such as the [ESP-Prog-2](https://docs.espressif.com/projects/esp-dev-kits/en/latest/other/esp-prog-2/user_guide.html#).

The `spincoater` program has a cargo feature that uses pins **23** and **22** for TX and RX instead. You can run it with `cargo run --bin spincoater_with_pc -F uart_over_adapter`.

### ESC Workaround
We are currently using pin **15** as a constant output due to a hardware issue.
