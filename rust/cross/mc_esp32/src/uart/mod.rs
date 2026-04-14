/// The baud rate for UART communication.
///
/// This value was taken from [`esp_hal::uart::Config::default`]
/// and is placed here so [`esp_hal::uart::Config::default`] doesn't change it under our feet.
pub const BAUD_RATE: u32 = 115_200;
