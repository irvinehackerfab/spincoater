use static_cell::ConstStaticCell;

/// The buffer used for display pixels
pub static SPI_BUFFER: ConstStaticCell<[u8; 512]> = ConstStaticCell::new([0u8; 512]);
