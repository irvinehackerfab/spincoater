//! This crate provides a TUI for the PC connecting to the spincoater's ESP32.
pub mod app;

use std::net::{Ipv4Addr, SocketAddrV4};

use cfg_if::cfg_if;
use color_eyre::Result;
use tokio::net::TcpStream;

use crate::app::App;

cfg_if! {
    if #[cfg(feature = "dev-socket")] {
        pub(crate) const DEV_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080);

        #[tokio::main]
        async fn main() -> Result<()> {
            color_eyre::install()?;
            open_dev_connection()?;
            let stream = TcpStream::connect(DEV_ADDRESS).await?;
            let terminal = ratatui::init();
            let result = App::new(stream)?.run(terminal).await;
            ratatui::restore();
            result
        }

        /// Binds a TCP socket to [`DEV_ADDRESS`] and spawns a task to accept all messages.
        fn open_dev_connection() -> Result<()> {
            use crate::DEV_ADDRESS;
            use crate::app::event::BUFFER_SIZE;
            use tokio::io::AsyncReadExt;
            use tokio::net::TcpSocket;

            let socket = TcpSocket::new_v4()?;
            socket.bind(DEV_ADDRESS.into())?;
            let listener = socket.listen(0)?;
            // Open fake MCU socket
            tokio::spawn(async move {
                'connection: loop {
                    let mut stream = listener
                        .accept()
                        .await
                        .expect("Failed to accept connection")
                        .0;
                    let mut buffer = [1u8; BUFFER_SIZE];
                    loop {
                        match stream.read(&mut buffer).await {
                            Ok(0) | Err(_) => continue 'connection,
                            Ok(_) => {
                            }
                        }
                    }
                }
            });
            Ok(())
        }
    } else {
        /// Keep this up to date with the IP and port listed in `../cross/mc_esp32/src/wifi/mod.rs`
        pub(crate) const MCU_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(192, 168, 2, 1), 8080);

        #[tokio::main]
        async fn main() -> Result<()> {
            color_eyre::install()?;
            println!("Attempting to connect to the MCU. If you're not connected to the wifi, connect and restart the program.");
            let stream = TcpStream::connect(MCU_ADDRESS).await?;
            let terminal = ratatui::init();
            let result = App::new(stream)?.run(terminal).await;
            ratatui::restore();
            result
        }
    }
}
