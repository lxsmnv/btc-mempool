
use std::io::{BufReader, Error, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use bitcoin::consensus::{Decodable, encode};
use bitcoin::io::ErrorKind::WouldBlock;
use bitcoin::Network;
use bitcoin::p2p::message::{NetworkMessage, RawNetworkMessage};
use bitcoin::p2p::message_network::VersionMessage;
use bitcoin::p2p::{Address, ServiceFlags};
use slog::{info, trace, Logger};
use tokio::io;
use crate::p2p::mempool_info::MempoolInfo;

fn connect_to_peer(addr: &SocketAddr) -> io::Result<(TcpStream, BufReader<TcpStream>)> {
    let writer = TcpStream::connect_timeout(addr, Duration::from_secs(3))?;
    writer.set_read_timeout(Some(Duration::from_millis(200)))?;
    let reader = writer.try_clone()?;
    let stream_reader = BufReader::new(reader);
    return Ok((writer, stream_reader));
}

pub async fn query_mempool(logger: &Logger, addr: SocketAddr, duration: Duration) -> io::Result<MempoolInfo> {
    let start_time = SystemTime::now();
    info!(logger, "query_mempool started, ip: {}", addr);
    let (mut writer,  mut stream_reader) = connect_to_peer(&addr)?;

    writer.write_all(&encode::serialize(&build_version_message(addr)).as_slice())?;
    let mempool_info = MempoolInfo::new(addr.ip());
    let mut state: (Option<NetworkMessage>, Option<MempoolInfo>) = (None, None);

    loop {
        let current_time = SystemTime::now();
        if current_time.duration_since(start_time).unwrap() > duration {
            match &state {
                (_, Some(mi)) => { return Ok(mi.clone()); }
                (_, None) => { return Err(Error::new(io::ErrorKind::TimedOut, "Operation timed out")); }
            }
        }

        let reply = match RawNetworkMessage::consensus_decode(&mut stream_reader) {
          Ok(reply) => Ok(reply),
          Err(encode::Error::Io(ref e)) if e.kind() == WouldBlock => {
            if state.0 != None {
                let ping_message = RawNetworkMessage::new(Network::Bitcoin.magic(), NetworkMessage::Ping(0));
                trace!(logger,"Sending ping message");
                writer.write_all(&encode::serialize(&ping_message).as_slice())?;
            }
            continue;
          }
          Err(encode::Error::Io(_)) if state.1.is_some() => {
            return Ok(state.1.unwrap());
          }
          Err(e) =>
               Err(Error::new(io::ErrorKind::InvalidData, e.to_string()))
        }?;

        state.0 = Some(reply.payload().clone());

        match &state {
            (Some(NetworkMessage::Version(_)), _) => {
                let verack_message = build_verack_message();
                writer.write_all(&encode::serialize(&verack_message).as_slice())?;
                info!(logger,"Sending mempool message");
                let mempool_message = build_mempooll_message();
                writer.write_all(&encode::serialize(&mempool_message).as_slice())?;
            }
            (Some(NetworkMessage::Ping(ping)), _) => {
                let pong_message = RawNetworkMessage::new(Network::Bitcoin.magic(), NetworkMessage::Pong(*ping));
                trace!(logger, "Received pong message: {}", ping);
                writer.write_all(&encode::serialize(&pong_message).as_slice())?;
            }
            (Some(NetworkMessage::Pong(pong)), _) => {
                trace!(logger, "Received pong message: {}", pong);
            }
            (Some(m @ NetworkMessage::Verack), _) => {
                info!(logger,"Received verack message: {:?}", m);
                state.1 = Some(mempool_info);
            }
            (Some(NetworkMessage::FeeFilter(fee_filter)), Some(mut mi)) => {
                info!(logger, "Received fee filter message: {:?}", fee_filter);
                mi.set_fee_filter(*fee_filter as u64);
                state.1 = Some(mi);
            }
            (Some(NetworkMessage::GetHeaders(_)), _) => {
                info!(logger, "Received get headers message");
            }
            (Some(NetworkMessage::Inv(inv)), Some(mut mi)) => {
                info!(logger, "Received inv message with {:?} transactions", inv.len());
                mi.update_mempool_count(inv.len());
                state.1 = Some(mi);
            }
            (Some(message), _) => {
                info!(logger, "Received unknown message: {:?}", message);
            }
            _ => {
                info!(logger, "Invalid state");
                return Err(Error::new(io::ErrorKind::TimedOut, "Operation timed out"));
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

fn build_verack_message() -> RawNetworkMessage {
    return RawNetworkMessage::new(Network::Bitcoin.magic(), NetworkMessage::Verack)
}