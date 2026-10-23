# Linear Regression
This is a binary that you can run to convert a log file from [host_tui](../host_tui) into a slope intercept equation for converting from motor RPM to duty cycle.

The output values are optimized for the `u16` arithmetic that we use in the spincoater firmware.

Run with `cargo run --bin linear_regression`.
