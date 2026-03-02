# Rust for Spin Coater
This is the workspace for device-agnostic libraries and Rust programs that run on the host PC.

Programs that run on the spin coater's microcontrollers can be found in the `cross` folder.

# Configuring your editor
Some of the crates in this workspace have templates for code editor configurations that allow diagnostics, lints, etc to show up while you code.

For Zed---and possibly other code editors---it will apply the first config it sees in the working directory. If you want to apply a config only to one crate, any other crates must be opened in their own directory, or else the config will apply to them as well.
