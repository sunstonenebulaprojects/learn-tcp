use etherparse::{Ipv4Header, PacketHeaders, TcpHeader};
use state_machine::AsyncTun;
use std::sync::Arc;
use tracing::warn;

#[allow(dead_code)]
pub fn parse_buffer<'a>(buffer: &'a [u8]) -> Option<(PacketHeaders<'a>, Ipv4Header, TcpHeader)> {
    let proto = u16::from_be_bytes([buffer[2], buffer[3]]);

    if proto != 0x800
    /*Not IPv4 packets*/
    {
        warn!(proto, "Not Ipv4 packet");
        return None;
    }
    let p = match etherparse::PacketHeaders::from_ip_slice(&buffer[4..]) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Skipping weird packet: {:?}", e);
            return None;
        }
    };
    let (ip, tcp) = match (&p.ip, &p.transport) {
        (
            Some(etherparse::IpHeader::Version4(ip, _)),
            Some(etherparse::TransportHeader::Tcp(tcp)),
        ) => (ip, tcp),
        _ => {
            log::warn!("Failed to parse Ipv4 and Tcp header from {:#?}", p);
            return None;
        }
    };
    if ip.protocol != 0x06
    /*Not tcp packet*/
    {
        warn!(ip_protocol = ip.protocol, "Not Tcp packet");
        return None;
    }

    Some((p.clone(), ip.clone(), tcp.clone()))
}

pub async fn read<'a>(nic: &Arc<dyn AsyncTun + Sync + Send>) -> Vec<u8> {
    let mut buffer = vec![0; 1504];
    let nbytes = nic.read(&mut buffer).await.unwrap();

    buffer[..nbytes].to_vec()
}
