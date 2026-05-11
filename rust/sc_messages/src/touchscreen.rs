//! This module describes the touchscreen data passed between the devices.

use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

/// A point where the screen was touched.
///
/// X and Y values are in the range 0..4095.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
    pub z1: u16,
    pub z2: u16,
}

impl TouchPoint {
    /// Creates a touchpoint.
    pub fn new(x: u16, y: u16, z1: u16, z2: u16) -> Self {
        Self { x, y, z1, z2 }
    }

    /// Swaps x and y.
    /// Useful for landscape displays.
    pub fn with_transpose(self) -> Self {
        Self {
            x: self.y,
            y: self.x,
            z1: self.z1,
            z2: self.z2,
        }
    }
}

// impl From<Point> for TouchPoint {
//     /// Truncates and removes the sign to fit in a [`u16`].
//     ///
//     /// Data will never be lost since values are in the range 0..4095.
//     #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
//     fn from(value: Point) -> Self {
//         Self {
//             x: value.x as u16,
//             y: value.y as u16,
//         }
//     }
// }
