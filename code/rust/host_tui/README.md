# Host Terminal User Interface
This is a Rust binary that you run on your PC while connected to the microcontroller's USB port. All motor data received from the microcontoller is written to a log file in the `logs/motor_data` folder of the executable's directory. The log file's name is the current date when starting the executable (e.g. `2026-02-10.csv`). If the file already exists, `_(1)` or `_(2)` or etc. is added to the name.

When writing motion profile CSV files, you must have the headers `rpm,time (micros)`. Do not set an rpm at time 0.

You can run it with `cargo run --bin host_tui`.

The parent directory has a Nix flake output for building host_tui for Windows from any OS. To use it, [install Nix](https://nixos.org/download/) and in the [parent](../) directory, run `nix build .#host_tui_windows`. The `.exe` will be in `./result/bin/`.
