use std::net::{Ipv4Addr, SocketAddrV4};

use cfg_if::cfg_if;
use tokio::net::TcpStream;

use crate::app::App;

pub mod app;
pub mod event;
pub mod ui;

#[tokio::main]
#[cfg(not(debug_assertions))]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    println!(
        "Attemping to connect to the MCU. If the TUI does not appear, please make sure you are on the MCU's wifi/bluetooth."
    );
    let stream = TcpStream::connect(MCU_ADDRESS).await?;
    let terminal = ratatui::init();
    let result = App::new(stream).run(terminal).await;
    ratatui::restore();
    result
}

#[tokio::main]
#[cfg(debug_assertions)]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let socket = TcpSocket::new_v4().expect("Failed to create socket");
    socket
        .bind(DEV_ADDRESS.into())
        .expect("Failed to bind socket");
    let listener = socket.listen(0).expect("Failed to listen");
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

    // Open fake connection
    let stream = TcpStream::connect(DEV_ADDRESS).await?;
    let terminal = ratatui::init();
    let result = App::new(stream).run(terminal).await;
    ratatui::restore();
    result
}

cfg_if! {
    if #[cfg(debug_assertions)] {
        use crate::event::BUFFER_SIZE;
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpSocket,
        };

        const DEV_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080);
    } else {
        // Keep this up to date with ../cross/mc_esp32/src/bin/wifi_pwm/wifi/mod.rs
        pub(crate) const MCU_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(192, 168, 2, 1), 8080);
    }
}
