# Motor Controller: ESP32
This is a [Rust](https://rust-lang.org/) program that you flash onto the [ESP32](https://www.espressif.com/en/products/socs/esp32) with [espflash](https://docs.espressif.com/projects/rust/book/getting-started/tooling/espflash.html).

The guide for Rust programming on ESP32 can be found [here](https://docs.espressif.com/projects/rust/book/preface.html).

This was generated from [esp-generate](https://docs.espressif.com/projects/rust/book/getting-started/tooling/esp-generate.html).

The `editor_configurations` folder contains default configurations for various editors. To avoid conflicting with any configurations you may have, they have no effect until you move them out into the `esp32` folder and add a `.` to the front of the name.

## Note for flashing the program
When flashing the program, you must specify the Wifi SSID and password through environment variables. One way to do this is by running the program with `SSID=_ PASSWORD=_ cargo run --release`
