use std::sync::Arc;

use crate::receiver::{self, parse_buffer};
use crossbeam_channel::{Receiver, Select, Sender};
use state_machine::{AsyncTun, Connection, ConnectionResult, Quad};
use std::net::Ipv4Addr;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument};

enum UserInput {
    CloseCommand,
    SendData(Vec<u8>),
}

#[instrument(skip_all)]
async fn client_loop(
    mut connection: Connection,
    quad: Quad,
    from_user: Receiver<UserInput>,
    from_server: Receiver<Vec<u8>>,
    handle1: &JoinHandle<()>,
    handle2: &JoinHandle<()>,
) {
    let mut sel = Select::new();
    sel.recv(&from_user);
    sel.recv(&from_server);
    const FROM_USER: usize = 0;
    const FROM_SERVER: usize = 1;

    loop {
        let index = sel.ready();
        match index {
            FROM_USER => match from_user.try_recv() {
                Ok(UserInput::CloseCommand) => {
                    connection.close(quad.clone()).await.unwrap();
                }
                Ok(UserInput::SendData(data)) => {
                    connection.send(quad.clone(), data.to_vec()).await.unwrap();
                }
                Err(_) => { /* TODO: decide */ }
            },
            FROM_SERVER => {
                match from_server.try_recv() {
                    Ok(buffer) => {
                        if let Some((p, iph, tcph)) = parse_buffer(&buffer) {
                            info!(
                                tcph.ack,
                                tcph.syn,
                                tcph.psh,
                                tcph.fin,
                                tcph.rst,
                                ack = tcph.acknowledgment_number,
                                sn = tcph.sequence_number,
                                src_ip = Ipv4Addr::from(iph.source).to_string(),
                                src_port = tcph.source_port,
                                dst_port = tcph.destination_port,
                                "Incoming segment"
                            );
                            match connection
                                .on_segment(iph.clone(), tcph.clone(), p.payload.to_vec())
                                .await
                            {
                                Ok(Some(ConnectionResult::Closed)) => {
                                    for _ in 0..3 {
                                        handle1.abort();
                                        handle2.abort();
                                        if handle1.is_finished() && handle2.is_finished() {
                                            break;
                                        }
                                        tokio::time::sleep(std::time::Duration::from_millis(200))
                                            .await;
                                    }
                                    if !(handle1.is_finished() && handle2.is_finished()) {
                                        error!(
                                            input_from_user = handle1.is_finished(),
                                            recv_task = handle2.is_finished(),
                                            "Failed to abort tasks"
                                        );
                                    }
                                    return;
                                }
                                Ok(Some(ConnectionResult::AckReceived)) => {
                                    info!("Ack received");
                                }
                                Ok(None) => {}
                                Err(_) => {}
                            }
                        }
                    }
                    Err(_) => { /* TODO: decide on error */ }
                }
            }
            _ => {}
        }
    }
}

async fn recv(nic: Arc<dyn AsyncTun + Sync + Send>, sender: Sender<Vec<u8>>) {
    loop {
        let buffer = { receiver::read(&nic).await };
        log::debug!("Received segment from server");
        sender
            .send(buffer)
            .expect("Failed to send buffer received from the server to a channel");
    }
}

async fn user_input(sender: Sender<UserInput>) {
    loop {
        let mut buffer = String::new();
        async_std::io::stdin()
            .read_line(&mut buffer)
            .await
            .expect("Failed to read from stdin");
        if buffer.starts_with("exit") {
            sender
                .send(UserInput::CloseCommand)
                .expect("Failed to send data from the user to the channel");
            return;
        } else {
            sender
                .send(UserInput::SendData(buffer.as_bytes().to_vec()))
                .expect("Failed to send data from the user to the channel");
        };
    }
}

#[instrument(skip_all)]
pub async fn run(quad: Quad, nic: Arc<dyn AsyncTun + Sync + Send>) {
    let mut connection = Connection::new(nic.clone());
    connection.open(quad.clone()).await.unwrap();

    let (to_user, from_user) = crossbeam_channel::unbounded();
    let (segment_sender, segment_receiver) = crossbeam_channel::unbounded();

    let nic_cloned = nic.clone();

    let cloned_end = to_user.clone();
    let handle1 = tokio::task::Builder::new()
        .name("user_input")
        .spawn(async move {
            user_input(to_user).await;
        });

    let handle2 = tokio::task::Builder::new()
        .name("tun_device_recv")
        .spawn(async move {
            recv(nic_cloned, segment_sender).await;
        });

    ctrlc::set_handler(move || {
        println!("Ctrl-C received");
        cloned_end
            .send(UserInput::CloseCommand)
            .expect("Failed to send data from a user to the channel");
    });

    client_loop(
        connection,
        quad,
        from_user,
        segment_receiver,
        &handle1,
        &handle2,
    )
    .await;
}
