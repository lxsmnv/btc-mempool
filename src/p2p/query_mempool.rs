
use std::io::{BufReader, Error, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use bitcoin::consensus::{Decodable, encode};
use bitcoin::io::ErrorKind::WouldBlock;
use bitcoin::Network;
use bitcoin::p2p::message::{NetworkMessage, RawNetworkMessage};
use bitcoin::p2p::message_network::VersionMessage;
use bitcoin::p2p::{Address, ServiceFlags};
use log::{info, trace};
use tokio::io;
use crate::p2p::mempool_info::MempoolInfo;

fn connect_to_peer(addr: &SocketAddr) -> io::Result<(TcpStream, BufReader<TcpStream>)> {
    let writer = TcpStream::connect_timeout(addr, Duration::from_secs(3))?;
    writer.set_read_timeout(Some(Duration::from_millis(200)))?;
    let reader = writer.try_clone()?;
    let stream_reader = BufReader::new(reader);
    return Ok((writer, stream_reader));
}

pub async fn query_mempool( addr: SocketAddr, duration: Duration) -> io::Result<MempoolInfo> {
    let start_time = SystemTime::now();
    info!("query_mempool started, ip: {}", addr);
    let (mut writer,  mut stream_reader) = connect_to_peer(&addr)?;

    writer.write_all(&encode::serialize(&build_version_message(addr)).as_slice())?;
    let mut mempool_state: Option<MempoolInfo> = None;

    loop {
        if SystemTime::now().duration_since(start_time).unwrap() > duration {
            match mempool_state {
                Some(mi) => return Ok(mi),
                None => return Err(Error::new(io::ErrorKind::TimedOut, "Operation timed out"))
            }
        }

        let reply = match RawNetworkMessage::consensus_decode(&mut stream_reader) {
          Ok(reply) => Ok(reply),
          Err(encode::Error::Io(ref e)) if e.kind() == WouldBlock => continue,
          Err(encode::Error::Io(_)) if mempool_state.is_some() => return Ok(mempool_state.unwrap()),
          Err(e) => Err(Error::new(io::ErrorKind::InvalidData, e.to_string()))
        }?;

        match (Some(reply.payload()), mempool_state.as_mut()) {
            (Some(NetworkMessage::Version(_)), _) => {
                let verack_message = build_verack_message();
                writer.write_all(&encode::serialize(&verack_message).as_slice())?;
                info!("Sending mempool message");
                let mempool_message = build_mempooll_message();
                writer.write_all(&encode::serialize(&mempool_message).as_slice())?;
            }
            (Some(NetworkMessage::Ping(ping)), _) => {
                trace!("Received ping message: {}", ping);
                let pong_message = build_pong_message(*ping);
                writer.write_all(&encode::serialize(&pong_message).as_slice())?;
            }
            (Some(m @ NetworkMessage::Verack), _) => {
                info!("Received verack message: {:?}", m);
                mempool_state = Some(MempoolInfo::new(addr.ip()));
            }
            (Some(NetworkMessage::FeeFilter(fee_filter)), Some(ref mut mi)) => {
                info!("Received fee filter message: {:?}", fee_filter);
                mi.set_fee_filter(*fee_filter as u64);
            }
            (Some(NetworkMessage::GetHeaders(_)), _) => {
                trace!("Received get headers message");
            }
            (Some(NetworkMessage::Inv(inv)), Some(ref mut mi)) => {
                info!("Received inv message with {:?} transactions", inv.len());
                mi.update_mempool_count(inv.len());
            }
            (Some(message), _) => {
                info!("Received unknown message: {:?}", message);
            }
            _ => {
                info!("Invalid state");
                return Err(Error::new(io::ErrorKind::InvalidData, "Invalid state"));
            }
        }
    }
}

fn build_version_message(addr: SocketAddr) -> RawNetworkMessage {
    let my_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);
    let version_message = VersionMessage {
        version: 70015,
        services: ServiceFlags::NONE, //
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
        receiver: Address::new(&addr, ServiceFlags::NETWORK),
        sender: Address::new(&my_address, ServiceFlags::NETWORK),
        nonce: 0,
        user_agent: "/bitcoin-rust:0.32.0/".to_string(),
        start_height: 0,
        relay: false,
    };
    return RawNetworkMessage::new(Network::Bitcoin.magic(), NetworkMessage::Version(version_message))
}

fn build_mempooll_message() -> RawNetworkMessage {
   return RawNetworkMessage::new(Network::Bitcoin.magic(), NetworkMessage::MemPool)
}

fn build_pong_message(nonce: u64) -> RawNetworkMessage {
    return RawNetworkMessage::new(Network::Bitcoin.magic(), NetworkMessage::Pong(nonce))
}

fn build_verack_message() -> RawNetworkMessage {
    return RawNetworkMessage::new(Network::Bitcoin.magic(), NetworkMessage::Verack)
}