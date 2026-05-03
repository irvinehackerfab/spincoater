# Rust for Spin Coater
This is the workspace for device-agnostic libraries and [Rust](https://rust-lang.org/) programs that run on the host PC.

Programs that run on the spin coater's microcontrollers can be found in the `cross` folder.

To develop these programs, either [install Rust](https://rust-lang.org/tools/install/) or use the dev environment provided by and documented in [DEVELOPMENT.md](DEVELOPMENT.md).

# Configuring your editor
The `editor_configurations` folder contains default configurations for various editors. To avoid conflicting with any configurations you may have, they have no effect until you move them out into the `rust` directory and add a `.` to the front of the name.
