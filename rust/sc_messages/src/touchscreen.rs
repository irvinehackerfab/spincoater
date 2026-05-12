//! This module describes the touchscreen data passed between the devices.

use embedded_graphics_core::geometry::Point;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

/// A point where the screen was touched.
///
/// X and Y values are in the range 0..4095.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct TouchPoint {
    pub x: u16,
    pub y: u16,
}

impl TouchPoint {
    /// Creates a touchpoint.
    #[must_use]
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    /// Swaps x and y.
    /// Useful for landscape displays.
    #[must_use]
    pub fn transpose(self) -> Self {
        Self {
            x: self.y,
            y: self.x,
        }
    }
}

impl From<Point> for TouchPoint {
    /// Truncates and removes the sign to fit in a [`u16`].
    ///
    /// Data will never be lost since values are in the range 0..4095.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn from(value: Point) -> Self {
        Self {
            x: value.x as u16,
            y: value.y as u16,
        }
    }
}
