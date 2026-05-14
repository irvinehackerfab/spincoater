# Host Terminal User Interface
<<<<<<< HEAD
This is a Rust binary that you run on your PC while connected to the microcontroller's USB port. All motor data received from the microcontoller is written to a log file in the `motor_data` folder of the executable's directory. The log file's name is the current date when starting the executable (e.g. `2026-02-10.txt`). If the file already exists, `_(1)` or `_(2)` or etc. is added to the name.
=======
This is a Rust binary that you run on your PC while connected to the microcontroller's USB port. All motor data received from the microcontoller is written to a log file in the `motor_data` folder of the executable's directory. The log file's name is the current date when starting the executable (e.g. `2026-02-10.csv`). If the file already exists, `_(1)` or `_(2)` or etc. is added to the name.
>>>>>>> main

When writing motion profile CSV files, you must have the headers `rpm,time (micros)`. Do not set an rpm at time 0.

<<<<<<< HEAD
You can run it with `cargo run`.
=======
Note that sending two rpm values with the same time will result in one of them being chosen at random.

You can run it with `cargo run --bin host_tui`.
>>>>>>> main
