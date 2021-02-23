use anyhow::Result;
use std::{env, io, str};
use udp::UdpSocket;

const BUFFER_SIZE: usize = 65535;
const CLIENT_PORT: u16 = 15000;

/// # エコーサーバ・クライアント
/// ## 実行例
/// ### 50000ポートで待ち受ける
/// `./udp server 50000`
/// ### ローカルの50000ポートに接続する
/// `./udp client localhost:50000`
fn main() -> Result<()> {
    let args = env::args().collect::<Vec<String>>();
    let role = &args[1];
    match role.as_str() {
        "server" => {
            let listen_port = args[2].parse::<u16>()?;
            echo_server(listen_port)
        }
        "client" => {
            let dest = &args[2];
            echo_client(dest)
        }
        _ => anyhow::bail!("Please specify server|client"),
    }
}

fn echo_server(port: u16) -> Result<()> {
    let mut socket = UdpSocket::new(port)?;
    loop {
        let mut buffer = [0; BUFFER_SIZE];
        let (size, src) = socket.recv_from(&mut buffer)?;
        print!("{}", str::from_utf8(&buffer[..size])?);
        socket.send_to(&buffer[..size], src)?;
    }
}

fn echo_client(dest: &str) -> Result<()> {
    let mut socket = UdpSocket::new(CLIENT_PORT)?;
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        socket.send_to(input.as_bytes(), dest)?;

        let mut buffer = [0; BUFFER_SIZE];
        let (n, _) = socket.recv_from(&mut buffer)?;
        print!("{}", str::from_utf8(&buffer[..n])?)
    }
}
