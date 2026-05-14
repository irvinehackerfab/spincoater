//! This crate contains functionality used by the host terminal user interface.

pub mod app;

use postcard_rpc::header::VarSeqKind;

/// The size of the outgoing queue.
pub const TX_QUEUE_SIZE: usize = 128;

/// The size of sequuence numbers used when making requests.
///
/// [`postcard_rpc`] gives no hint as to what this should be.
pub const VAR_SEQUENCE_KIND: VarSeqKind = VarSeqKind::Seq2;

/// The Vendor ID that shows up when you connect an ESP32 `DevKitC` to a PC over USB.
pub const DEV_KIT_C_VENDOR_ID: u16 = 4292;

/// The Vendor ID that shows up when you connect an ESP-Prog-2 to a PC over USB.
pub const ESP_PROG_2_VENDOR_ID: u16 = 12346;
