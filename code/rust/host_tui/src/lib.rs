//! This crate contains functionality used by the host terminal user interface.

pub mod app;

/// The Vendor ID that shows up when you connect an ESP32 `DevKitC` to a PC over USB.
pub const DEV_KIT_C_VENDOR_ID: u16 = 4292;

/// The Vendor ID that shows up when you connect an ESP-Prog-2 to a PC over USB.
pub const ESP_PROG_2_VENDOR_ID: u16 = 12346;
