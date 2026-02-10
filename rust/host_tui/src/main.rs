use std::net::{Ipv4Addr, SocketAddrV4};

use cfg_if::cfg_if;

use crate::app::App;

pub mod app;
pub mod event;
pub mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().await?.run(terminal).await;
    ratatui::restore();
    result
}

cfg_if! {
    if #[cfg(debug_assertions)] {
        pub(crate) const DEV_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080);
    } else {
        // Keep this up to date with ../cross/mc_esp32/src/bin/wifi_pwm/wifi/mod.rs
        pub(crate) const MCU_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(192, 168, 2, 1), 8080);
    }
}
