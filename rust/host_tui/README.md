# Host Terminal User Interface
This is a Rust binary that you run on your PC while connected to the microcontroller's Wifi. All messages received from the microcontoller are written to a log file in the `motor_data` folder of the executable's directory. The log file's name is the current date when starting the executable (e.g. `2026-02-10.txt`). If the file already exists, `_(1)` or `_(2)` or etc. is added to the name.

When writing motion profile CSV files, you must have the headers `rpm,time (micros)`.

You can run it with `cargo run`.
