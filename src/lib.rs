use anyhow::{Context, Result};
use pnet::packet::{
    ip::IpNextHeaderProtocols,
    udp::{self, MutableUdpPacket},
    Packet,
};
use pnet::transport::{
    self, TransportChannelType, TransportProtocol, TransportReceiver, TransportSender,
};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};

const UDP_HEADER_SIZE: usize = 8;
const BUFFER_SIZE: usize = 65535;
const LOCAL_ADDR: &str = "127.0.0.1";

pub struct UdpSocket {
    port: u16,
    sender: TransportSender,
    receiver: TransportReceiver,
}

impl UdpSocket {
    // Socketの初期化
    pub fn new(port: u16) -> Result<Self> {
        // channel の生成
        let (sender, receiver) = transport::transport_channel(
            BUFFER_SIZE,
            TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp)),
        )?;
        Ok(Self {
            port,
            sender,
            receiver,
        })
    }

    // 指定した宛先にUDPデータを送信する
    pub fn send_to<T: ToSocketAddrs>(&mut self, payload: &[u8], dest: T) -> Result<usize> {
        let total_length = UDP_HEADER_SIZE + payload.len();
        let mut buffer = vec![0; total_length];
        let mut packet = MutableUdpPacket::new(&mut buffer).context("failed to create packet")?;
        let dest = match dest
            .to_socket_addrs()?
            .next()
            .context("invalid destination")?
        {
            SocketAddr::V4(addr) => addr,
            SocketAddr::V6(_) => anyhow::bail!("IPv6 address is not supported"),
        };
        // 送信元port番号
        packet.set_source(self.port);
        // 宛先ポート番号
        packet.set_destination(dest.port());
        // UDPデータグラムのペイロードを含めた全長。 単位はoctet
        packet.set_length(total_length as u16);
        // payroad
        packet.set_payload(payload);
        //check sum
        packet.set_checksum(udp::ipv4_checksum(
            &packet.to_immutable(),
            &LOCAL_ADDR.parse::<Ipv4Addr>()?,
            dest.ip(),
        ));
        self.sender
            .send_to(packet, IpAddr::from(*dest.ip()))
            .context("failed to send")
    }

    pub fn recv_from(&mut self, mut buffer: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let mut packet_iter = transport::udp_packet_iter(&mut self.receiver);
        loop {
            if let Ok((udp_packet, IpAddr::V4(src_addr))) = packet_iter.next() {
                // ソケットに紐づくポート意外に到達したパケットは無視する
                if self.port != udp_packet.get_destination() {
                    continue;
                }
                // チェックサムの検証
                if udp_packet.get_checksum() != 0
                    && udp_packet.get_checksum()
                        != udp::ipv4_checksum(
                            &udp_packet,
                            &src_addr,
                            &LOCAL_ADDR.parse::<Ipv4Addr>()?,
                        )
                {
                    continue;
                }
                let n = io::copy(&mut udp_packet.payload(), &mut buffer)? as usize;
                // 読み込んだバイト数と送信元のソケットアドレスを返す
                return Ok((
                    n,
                    SocketAddr::new(IpAddr::V4(src_addr), udp_packet.get_source()),
                ));
            }
        }
    }
}
