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
pub(crate) const _MCU_ADDRESS: SocketAddrV4 =
    SocketAddrV4::new(Ipv4Addr::new(192, 168, 2, 1), 8080);
const DEV_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8080);

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    // Open dev connection
    tokio::spawn(async {
        let socket = TcpSocket::new_v4().unwrap();
        socket.bind(DEV_ADDRESS.into()).unwrap();
        let listener = socket.listen(1).unwrap();
        'connection: loop {
            let mut stream = listener.accept().await.unwrap().0;
            loop {
                let mut buffer = [0u8; 64];
                match stream.read(&mut buffer).await {
                    Ok(0) => continue 'connection,
                    Ok(_) => {}
                    Err(_) => continue 'connection,
                }
            }
        }
    });
    // println!(
    //     "Attemping to connect to the MCU. If the TUI does not appear, please make sure you are on the MCU's wifi/bluetooth."
    // );
    // let stream = TcpStream::connect(MCU_ADDRESS).await?;
    // Open dev connection
    let stream = TcpStream::connect(DEV_ADDRESS).await?;
    let terminal = ratatui::init();
    let result = App::new(stream).run(terminal).await;
    ratatui::restore();
    result
}
