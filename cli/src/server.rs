use crate::receiver::{parse_buffer, read};
use state_machine::{AsyncTun, Connection, ConnectionResult, IpAddrPort, Quad};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::{collections::hash_map::Entry, sync::Arc};
use tracing::{info, instrument};

#[instrument(name = "server_run", skip(nic))]
pub async fn run(nic: Arc<dyn AsyncTun + Sync + Send>) {
    let mut connections: HashMap<Quad, Connection> = HashMap::new();

    loop {
        let buffer = read(&nic).await;
        if let Some((p, ip, tcp)) = parse_buffer(&buffer) {
            let payload = String::from_utf8_lossy(p.payload);
            let payload: &str = payload.borrow();
            info!(
                tcp.ack,
                tcp.syn,
                tcp.psh,
                tcp.fin,
                tcp.rst,
                ack = tcp.acknowledgment_number,
                sn = tcp.sequence_number,
                payload,
                src_ip = Ipv4Addr::from(ip.source).to_string(),
                src_port = tcp.source_port,
                dst_port = tcp.destination_port,
                "Incoming segment"
            );
            let quad = Quad {
                src: IpAddrPort {
                    ip: Ipv4Addr::from(ip.source),
                    port: tcp.source_port,
                },
                dst: IpAddrPort {
                    ip: Ipv4Addr::from(ip.destination),
                    port: tcp.destination_port,
                },
            };
            match connections.entry(quad) {
                Entry::Occupied(mut o) => {
                    if let Ok(Some(ConnectionResult::Closed)) = o
                        .get_mut()
                        .on_segment(ip.clone(), tcp.clone(), p.payload.to_vec())
                        .await
                    {
                        println!("Client closed connection");
                        return;
                    }
                }
                Entry::Vacant(_) => {
                    info!("New connection: from: {:?}, to: {:?}", quad.src, quad.dst);
                    let mut c = Connection::new(nic.clone());
                    c.passive_open().await.unwrap();
                    connections.insert(quad, c);
                }
            }
        }
    }
}
