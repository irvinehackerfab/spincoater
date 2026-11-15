# UCI Hacker Fab Spin Coater

An open source spin coater built by [Irvine Hacker Fab](https://github.com/irvinehackerfab). It spins a wafer at a set
speed so that a liquid like photoresist spreads into a thin, even film.

This branch is **V3**, the current generation. It fits 3-4 inch wafers with a swappable vacuum chuck and drives a
standard RC brushless motor and ESC with a 50 Hz PWM signal from an ESP32. It can run alone from its touchscreen, or
from a PC with a terminal app that runs speed profiles and logs the motor data.

> [!WARNING]
> V3 is still being built. Most of the parts lists, photos, CAD exports and data in this repo are from earlier builds
> and will change. Check the [Onshape document](https://cad.onshape.com/documents/92eb9cb52198894acd4b607c/v/db3ac1a34d2a62ffda81dc34/e/fb45d9e7be92a57d799034dd)
> and the latest release before ordering parts or printing.

![V3 spin coater, May 2026](images/photos/2026-05-07-v3-assembled.jpg)

## Specs
These describe the current build and may change.

| | V3 |
|---|---|
| Wafer size | 3-4 in |
| Chuck | Swappable vacuum chuck, 12V mini vacuum pump (8 L/min) |
| Tested speeds | 500-5000 RPM, within 0.35% of the setpoint once settled ([test](data/2026-10-09-controller-comparison)) |
| Motor | HobbyStar 3300KV sensored 540 brushless motor, belt driven |
| Motor controller | HobbyStar TS80A sensored ESC, 50 Hz PWM |
| Controller board | ESP32-DevKitC-32E |
| Display | 2.8 in ILI9341 SPI touchscreen |
| Power input | 12V 10A DC adapter (5.5 x 2.5 mm barrel jack) |

## Which version should I build?
Each spin coater generation has its own branch. Download the files for your model from its latest
[release](https://github.com/irvinehackerfab/spincoater/releases) rather than from a branch, since a branch can have
untested changes.

| Model | Branch | Latest release | CAD |
|---|---|---|---|
| V3 (current) | `v3` | TODO (first release will be `v3.0.0`) | [Onshape V34 R1](https://cad.onshape.com/documents/92eb9cb52198894acd4b607c/v/db3ac1a34d2a62ffda81dc34/e/fb45d9e7be92a57d799034dd) |
| Older code (Arduino, MicroPython) | `legacy` | none | none |

## Repository layout
| Folder | What's in it |
|---|---|
| [`code/`](code) | Rust firmware for the ESP32 and the PC programs |
| [`hardware/`](hardware) | Onshape link, printable parts and the bill of materials |
| [`docs/`](docs) | Assembly, wiring, operation and calibration guides, and a photo build log |
| [`data/`](data) | Test logs and measurements |
| [`images/`](images) | Photos, CAD renders, diagrams and plots used by the docs |

## Quick start
1. Build the hardware: [hardware/](hardware) and [docs/assembly.md](docs/assembly.md).
2. Flash the firmware onto the ESP32: [code/](code).
3. Run a coat from the touchscreen or a PC: [docs/operation.md](docs/operation.md).

## Contributing
See [CONTRIBUTING.md](CONTRIBUTING.md) for how branches, tags and releases work.

## License
[MIT](LICENSE.txt)
