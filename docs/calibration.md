# Calibration

> [!WARNING]
> The controller and its constants are still being tuned, so the numbers here will change.

The ESC reads a 50 Hz PWM signal: a 7.5% duty cycle is neutral (stopped) and 10% is full power. The firmware has to
turn an RPM setpoint into a duty cycle in that range.

## Feedforward
We use a cubic equation that turns the setpoint RPM into a duty cycle.
1. Flash the [`spincoater_with_pc`](../code/rust/cross/esp32/README.md) program onto the spincoater.
2. Run [`host_tui`](../code/rust/host_tui/README.md).
3. Run a profile that increases linearly from 0 to 9,000 plate RPM for as long as possible. For example,
```csv
rpm,time (micros)
9000,60000000
```
4. A motor log will be placed in your current directory. Upload it to the [Scientific Data Graphing Lab](https://livephysics.com/labs/scientific-data-graphing-lab/).
5. Select `current_rpm` as the X column and `duty_cycle` as the Y column.
6. Select `Best Fit (auto)`
7. Copy the coefficients from the resulting equation to [`cubic_conversion`](../code/rust/cross/esp32/src/gpio/pwm.rs).

> [!NOTE]
> If you change the cubic coefficients to less accurate ones, the spincoater may fail to settle at its target RPM. Either revert your changes or increase [`I_LIMIT`](../code/rust/cross/esp32/src/pid.rs).

## Feedback
We also use a feedback controller with proportional, integral and derivative terms that corrects error using the RPM measured by the hall effect sensor.
The gains are in [`pid.rs`](../code/rust/cross/esp32/src/pid.rs), and you'll need to tune these through trial and error.
