use std::net::{Ipv4Addr, SocketAddrV4};

use tokio::{
    io::AsyncReadExt,
    net::{TcpSocket, TcpStream},
};

use crate::app::App;

pub mod app;
pub mod event;
pub mod ui;

// Keep this up to date with ../cross/mc_esp32/src/bin/wifi_pwm/wifi/mod.rs
#[cfg(not(debug_assertions))]
pub(crate) const MCU_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(192, 168, 2, 1), 8080);

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

#[cfg(debug_assertions)]
const DEV_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080);

#[tokio::main]
#[cfg(debug_assertions)]
async fn main() -> color_eyre::Result<()> {
    // Only imported when debug_assertions are enabled
    use crate::event::BUFFER_SIZE;
    use tokio::io::AsyncWriteExt;

    color_eyre::install()?;

    // Open fake MCU socket
    tokio::spawn(async {
        let socket = TcpSocket::new_v4().unwrap();
        socket.bind(DEV_ADDRESS.into()).unwrap();
        let listener = socket.listen(1).unwrap();
        'connection: loop {
            let mut stream = listener.accept().await.unwrap().0;
            let mut buffer = [1u8; BUFFER_SIZE];
            let mut pos = 0;
            loop {
                match stream.read(&mut buffer[pos..]).await {
                    Ok(0) => continue 'connection,
                    Ok(len) => {
                        pos += len;
                        if buffer.contains(&0u8) {
                            stream.write_all(&buffer[..pos]).await.unwrap();
                            buffer[..pos].iter_mut().for_each(|byte| *byte = 1u8);
                            pos = 0;
                        }
                    }
                    Err(_) => continue 'connection,
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
