# Host Terminal User Interface
This is a Rust program that you run on your PC while connected to the microcontroller's Wifi.

## Development Note
When running with the dev profile, a fake MCU socket will be created at localhost which receives and immediately sends back all messages.

To actually connect to the MCU, run `cargo run --release`.
