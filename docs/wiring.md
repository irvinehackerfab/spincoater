# Wiring

> [!WARNING]
> The electronics are still changing. These diagrams are drafts and may not match the current build.

The ESP32 pin layout is kept with the firmware so it stays in sync with the code:
[code/rust/cross/esp32/README.md](../code/rust/cross/esp32/README.md#pin-layout).

## Power board
Everything runs from a 12V 10A adapter. A buck converter steps it down to 5V for the ESP32, and an IRLZ44N MOSFET
switches the 12V vacuum pump from an ESP32 GPIO pin. The ESC gets 12V through the power switch and the E-stop.

Schematic:

![Power board schematic](../images/diagrams/power-board-schematic.png)

Hand-drawn perfboard layout:

![Power board layout](../images/diagrams/power-board-hand-drawn.jpg)

TODO: which of these two matches the current build.

## ESC sensor port
![ESC sensor port pinout](../images/photos/2025-11-03-esc-sensor-port-pinout.jpg)

## Inside the enclosure
| | |
|---|---|
| ![Wiring, side panel open](../images/photos/2026-05-08-wiring-side-panel-open.jpg) | ![Wiring, back panel open](../images/photos/2026-05-08-wiring-back-panel-open.jpg) |
| ![ESP32 perfboard](../images/photos/2026-05-07-esp32-perfboard.jpg) | ![Screen and board stack](../images/photos/2026-05-07-screen-and-board-stack.jpg) |

## Connections
TODO: ESC, motor, hall effect sensor, vacuum pump, touchscreen and the PC/UART connection.
