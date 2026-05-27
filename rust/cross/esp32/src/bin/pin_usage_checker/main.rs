#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_backtrace as _;
use esp_hal::clock::CpuClock;

fn _pins() {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // These GPIO pins are in use by some feature of the module and should not be used.
    // let _ = peripherals.GPIO6;
    // let _ = peripherals.GPIO7;
    // let _ = peripherals.GPIO8;
    // let _ = peripherals.GPIO9;
    // let _ = peripherals.GPIO10;
    // let _ = peripherals.GPIO11;

    // Pins reserved for the ESP-Prog-2
    let _jtag_tdi = peripherals.GPIO12;
    let _jtag_tck = peripherals.GPIO13;
    let _jtag_tms = peripherals.GPIO14;
    let _jtag_tdo = peripherals.GPIO15;

    // Pins reserved for UART flashing or UART communication
    let _flashing_boot = peripherals.GPIO0;
    let _tx_over_devkit = peripherals.GPIO1;
    let _rx_over_devkit = peripherals.GPIO3;
    let _tx_over_adapter = peripherals.GPIO23;
    let _rx_over_adapter = peripherals.GPIO22;

    // Pins reserved for the display
    let _display_chip_select = peripherals.GPIO19;
    let _reset = peripherals.GPIO18;
    let _data_command = peripherals.GPIO25;
    let _mosi = peripherals.GPIO33;
    let _clock = peripherals.GPIO32;
    let _miso = peripherals.GPIO35;
    let _touch_chip_select = peripherals.GPIO16;
    let _touch_interrupt_request = peripherals.GPIO34;

    // Pins used by programs
    let _motor_control_pwm = peripherals.GPIO26;
    let _motor_encoder_interrupt = peripherals.GPIO27;
    let _vacuum_pump_active_high = peripherals.GPIO17;
}
