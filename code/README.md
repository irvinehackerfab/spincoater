# Code

All of the V3 code is written in [Rust](https://rust-lang.org/) and lives in [`rust/`](rust).

| Program | Runs on | What it does |
|---|---|---|
| [`spincoater`](rust/cross/esp32/src/bin/spincoater) | ESP32 | Runs the spin coater from its touchscreen at one RPM for a set time |
| [`spincoater_with_pc`](rust/cross/esp32/src/bin/spincoater_with_pc) | ESP32 | Runs the spin coater from commands sent by a PC over UART |
| [`host_tui`](rust/host_tui) | PC | Terminal app that sends speed profiles to `spincoater_with_pc` and logs the motor data |
| [`linear_regression`](rust/linear_regression) | PC | Turns a `host_tui` log into constants for converting RPM to PWM duty cycle |
| [`pwm`](rust/cross/esp32/src/bin/pwm) | ESP32 | Holds a constant 7.5% duty cycle to check the ESC |
| [`touchscreen_test`](rust/cross/esp32/src/bin/touchscreen_test), [`touchscreen_calibration`](rust/cross/esp32/src/bin/touchscreen_calibration) | ESP32 | Touchscreen checks and calibration |
| [`sc_messages`](rust/sc_messages) | Both | Message types shared between the PC and the ESP32 |

## Setup
- PC programs: see [rust/README.md](rust/README.md).
- ESP32 firmware, pin layout and flashing: see [rust/cross/esp32/README.md](rust/cross/esp32/README.md).

## Older code
The Arduino and MicroPython versions of the firmware are kept on the
[`legacy`](https://github.com/irvinehackerfab/spincoater/tree/legacy) branch.
