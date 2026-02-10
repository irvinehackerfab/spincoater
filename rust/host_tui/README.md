# Host Terminal User Interface
This is a Rust program that you run on your PC while connected to the microcontroller's Wifi. All messages received from the microcontoller are written to a log file in the `sc_logs` folder of the executable's directory. The log file's name is the current date when starting the executable (e.g. `2026-02-10.txt`).

## Development Note
When running with the dev profile, a fake MCU socket will be created at localhost which receives and immediately sends back all messages.

To actually connect to the MCU, run `cargo run --release`.
