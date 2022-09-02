use crate::async_tun::AsyncTun;
use crate::quad::IpAddrPort;
use etherparse::{PacketBuilderStep, TcpHeader};
use std::io::Write;
use std::sync::Arc;
use tracing::{info, instrument};

#[instrument(skip(nic, to, from, window_size, sn), fields(serial_number = sn))]
pub async fn send_syn(
    nic: Arc<dyn AsyncTun + Sync + Send>,
    to: IpAddrPort,
    from: IpAddrPort,
    sn: u32,
    window_size: u16,
) {
    let builder = tcp_builder(
        to.ip.octets(),
        from.ip.octets(),
        to.port,
        from.port,
        sn,
        window_size,
    )
    .syn();
    send_tcp_packet(nic, builder, &[], "syn").await;
}

#[instrument(skip(nic, to, from, window_size, sn, ack), fields(serial_number = sn, acknowledgment_number = ack))]
pub async fn send_fin(
    nic: Arc<dyn AsyncTun + Sync + Send>,
    to: IpAddrPort,
    from: IpAddrPort,
    sn: u32,
    ack: u32,
    window_size: u16,
) {
    let builder = tcp_builder(
        to.ip.octets(),
        from.ip.octets(),
        to.port,
        from.port,
        sn,
        window_size,
    )
    .fin()
    .ack(ack);
    send_tcp_packet(nic, builder, &[], "fin").await;
}

#[instrument(skip_all, fields(serial_number = sn, acknowledgment_number = ack))]
pub async fn send_data(
    nic: Arc<dyn AsyncTun + Sync + Send>,
    to: IpAddrPort,
    from: IpAddrPort,
    sn: u32,
    ack: u32,
    window_size: u16,
    data: &[u8],
) {
    let builder = tcp_builder(
        to.ip.octets(),
        from.ip.octets(),
        to.port,
        from.port,
        sn,
        window_size,
    )
    .psh()
    .ack(ack);
    send_tcp_packet(nic, builder, data, "data").await;
}

#[instrument(skip(nic, iph, tcph, sn, ack_number), fields(serial_number = sn, acknowledgment_number = ack_number))]
pub async fn send_syn_ack(
    nic: Arc<dyn AsyncTun + Sync + Send>,
    iph: etherparse::Ipv4Header,
    tcph: etherparse::TcpHeader,
    sn: u32,
    ack_number: u32,
) {
    let builder = tcp_builder(
        iph.source,
        iph.destination,
        tcph.source_port,
        tcph.destination_port,
        sn,
        tcph.window_size,
    )
    .ack(ack_number)
    .syn();
    send_tcp_packet(nic, builder, &[], "syn_ack").await;
}

#[instrument(skip(nic, iph, tcph, sn, ack_number), 
    fields(
        serial_number = sn, 
        acknowledgment_number = ack_number, 
        dst_ip = ?iph.destination, 
        dst_port = tcph.destination_port,
        src_ip = ?iph.source, 
        src_port = tcph.source_port,
    ))]
pub async fn send_ack(
    nic: Arc<dyn AsyncTun + Sync + Send>,
    iph: etherparse::Ipv4Header,
    tcph: etherparse::TcpHeader,
    sn: u32,
    ack_number: u32,
) {
    let builder = tcp_builder(
        iph.source,
        iph.destination,
        tcph.source_port,
        tcph.destination_port,
        sn,
        tcph.window_size,
    )
    .ack(ack_number);
    send_tcp_packet(nic, builder, &[], "ack").await;
}

#[instrument(skip_all, fields(serial_number = sn, acknowledgment_number = ack_number))]
pub async fn send_fin_ack(
    nic: Arc<dyn AsyncTun + Sync + Send + '_>,
    iph: &etherparse::Ipv4Header,
    tcph: &etherparse::TcpHeader,
    sn: u32,
    ack_number: u32,
) {
    let builder = tcp_builder(
        iph.source,
        iph.destination,
        tcph.source_port,
        tcph.destination_port,
        sn,
        tcph.window_size,
    )
    .ack(ack_number)
    .fin();
    send_tcp_packet(nic, builder, &[], "fin_ack").await;
}

fn tcp_builder(
    dst_ip: [u8; 4],
    src_ip: [u8; 4],
    dst_port: u16,
    src_port: u16,
    sn: u32,
    window_size: u16,
) -> PacketBuilderStep<TcpHeader> {
    etherparse::PacketBuilder::ipv4(src_ip, dst_ip, 64).tcp(src_port, dst_port, sn, window_size)
}

#[instrument(skip_all)]
async fn send_tcp_packet(
    nic: Arc<dyn AsyncTun + Sync + Send + '_>,
    builder: etherparse::PacketBuilderStep<etherparse::TcpHeader>,
    payload: &[u8],
    packet_type: &str,
) {
    info!("Sending {} packet", packet_type);

    let mut result = vec![0; builder.size(payload.len()) + 4];    
    let _ = (&mut result[2..]).write(&0x800u16.to_be_bytes()).unwrap();
    let mut offset = &mut result[4..];
    builder.write(&mut offset, payload).unwrap();    
    if let Err(e) = nic.write(&result).await {
        println!("Failed to send tcp packet: {:?}", e);
    }
}
