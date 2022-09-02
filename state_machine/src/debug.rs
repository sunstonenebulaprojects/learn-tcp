use std::net::Ipv4Addr;

pub fn print_info(iph: &etherparse::Ipv4Header, tcph: &etherparse::TcpHeader, data: &[u8]) {
    println!(
        "{}:{} -> {}:{} {}b of tcp, data: {}",
        Ipv4Addr::from(iph.source),
        tcph.source_port,
        Ipv4Addr::from(iph.destination),
        tcph.destination_port,
        data.len(),
        String::from_utf8_lossy(data).trim_end()
    );
}

#[allow(dead_code)]
pub fn debug_packet_print(raw: &[u8]) {
    let p = etherparse::PacketHeaders::from_ip_slice(raw).unwrap();
    match (&p.ip, &p.transport) {
        (
            Some(etherparse::IpHeader::Version4(ip, _)),
            Some(etherparse::TransportHeader::Tcp(tcp)),
        ) => {
            println!("ip header: {:#?}", ip);
            println!("ACK header: {:#?}, raw: {:02x?}", tcp, raw);
        }
        _ => {}
    };
}
