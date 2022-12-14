mod client;
mod receiver;
mod server;

use clap::{Args, Parser, Subcommand};
use metrics_exporter_prometheus::PrometheusBuilder;
use state_machine::{AsyncIFace, AsyncTun, IpAddrPort, Quad};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Parser, Debug)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Client(Client),
}

#[derive(Args, Debug)]
struct Client {
    /// <IpV4Addr:Port>
    #[clap(long)]
    src: IpAddrPort,
    /// <IpV4Addr:Port>
    #[clap(long)]
    dst: IpAddrPort,
}

fn main() {
    let format = tracing_subscriber::fmt::format().pretty();
    tracing_subscriber::fmt().event_format(format).init();
    // console_subscriber::init();

    let cli = Cli::parse();

    let rt = Runtime::new().unwrap();

    rt.block_on(async move {
        let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 9091);
        PrometheusBuilder::new()
            .with_http_listener(socket)
            .install()
            .expect("Failed to install scrape endpoint for Prometheus");

        let nic: Arc<dyn AsyncTun + Sync + Send> = Arc::new(
            AsyncIFace::new(
                tun_tap::Iface::new("tun0", tun_tap::Mode::Tun)
                    .expect("failed to create a Tun device"),
            )
            .expect("Failed to create async interface for tun device"),
        );

        if let Some(Command::Client(client)) = &cli.command {
            println!("will work like a tcp client");

            client::run(
                Quad {
                    dst: client.dst,
                    src: client.src,
                },
                nic,
            )
            .await;
        } else {
            server::run(nic).await;
        }
    });
}
