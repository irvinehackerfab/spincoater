# Operation

> [!WARNING]
> The firmware and controls are still changing, so these steps may not match the latest release.

<!-- TODO: safety notes (lid, loose wafers, solvent fumes) -->

## Loading a wafer
TODO: choose the chuck for your wafer size, center the wafer and turn on the vacuum.

## From the touchscreen
Flash the [`spincoater`](../code/rust/cross/esp32/src/bin/spincoater) program. It runs the spin coater at one RPM
for a set number of seconds, both picked on the touchscreen.

TODO: screenshots of the screen.

## From a PC
1. Flash the [`spincoater_with_pc`](../code/rust/cross/esp32/src/bin/spincoater_with_pc) program.
2. Connect the PC to the ESP32's USB port and run [`host_tui`](../code/rust/host_tui).
3. Load a motion profile, which is a CSV of RPM setpoints over time. Examples are in
   [code/rust/motion_profiles](../code/rust/motion_profiles).

A motion profile must have the header `rpm,time (micros)`, and it can't set an RPM at time 0:
```csv
rpm,time (micros)
20000,1000
20000,30000000
```

`host_tui` logs all motor data to `logs/motor_data/<date>.csv` next to the executable.

TODO: whether the RPM in profiles is motor RPM or plate RPM (the pulleys change the ratio).

## Troubleshooting
- If the spin coater stops or won't run in PC mode, the motion profile probably caused an arithmetic error in
  [`motion_profile.rs`](../code/rust/cross/esp32/src/runners/motion_profile.rs).
- To check the ESC by itself, flash the [`pwm`](../code/rust/cross/esp32/src/bin/pwm) program, which holds the
  neutral 7.5% duty cycle.
