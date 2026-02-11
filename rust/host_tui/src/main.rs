use std::net::{Ipv4Addr, SocketAddrV4};

use cfg_if::cfg_if;
use color_eyre::Result;
use tokio::net::TcpStream;

use crate::app::App;

pub mod app;
pub mod event;
pub mod ui;

cfg_if! {
    if #[cfg(debug_assertions)] {
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

        /// Binds a TCP socket to [`DEV_ADDRESS`] and spawns a task to accept and send back all messages.
        fn open_dev_connection() -> Result<()> {
            use crate::DEV_ADDRESS;
            use crate::event::BUFFER_SIZE;
            use tokio::io::AsyncReadExt;
            use tokio::io::AsyncWriteExt;
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
                    let mut pos = 0;
                    loop {
                        match stream.read(&mut buffer[pos..]).await {
                            Ok(0) | Err(_) => continue 'connection,
                            Ok(len) => {
                                pos += len;
                                if buffer.contains(&0u8) {
                                    stream
                                        .write_all(&buffer[..pos])
                                        .await
                                        .expect("Failed to write to stream");
                                    buffer[..pos].iter_mut().for_each(|byte| *byte = 1u8);
                                    pos = 0;
                                }
                            }
                        }
                    }
                }
            });
            Ok(())
        }
    } else {
        // Keep this up to date with ../cross/mc_esp32/src/bin/wifi_pwm/wifi/mod.rs
        pub(crate) const MCU_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(192, 168, 2, 1), 8080);

        #[tokio::main]
        async fn main() -> Result<()> {
            color_eyre::install()?;
            println!("Attempting to connect to MCU. If you're not connected to the wifi, connect and restart the program.");
            let stream = TcpStream::connect(MCU_ADDRESS).await?;
            let terminal = ratatui::init();
            let result = App::new(stream)?.run(terminal).await;
            ratatui::restore();
            result
        }
    }
}
