//! This module describes the touchscreen data passed between the devices.

use embedded_graphics_core::geometry::Point;
use postcard_schema::Schema;
use serde::{Deserialize, Serialize};

/// A point where the screen was touched.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct TouchPoint {
    x: u16,
    y: u16,
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
