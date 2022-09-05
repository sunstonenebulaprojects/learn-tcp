#![allow(dead_code)]
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::states::{ClosedState, State};
use crate::transmission_control_block::{
    ReceiveSequenceVars, RetransmissionQueue, SendSequenceVars,
};

use crate::AsyncTun;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TransitionState(pub State);

#[async_trait]
pub trait HandleEvents {
    async fn on_segment(
        &mut self,
        iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>>;
    async fn passive_open(&mut self) -> TrustResult<Option<TransitionState>>;
    async fn open(&mut self, quad: Quad) -> TrustResult<Option<TransitionState>>;
    async fn close(&mut self, quad: Quad) -> TrustResult<Option<TransitionState>>;
    async fn send(&mut self, quad: Quad, data: Vec<u8>) -> TrustResult<Option<TransitionState>>;
}

pub enum ConnectionResult {
    Closed,
    AckReceived,
}

#[allow(dead_code)]
pub struct Connection {
    state: State,
}

impl Connection {
    pub fn new(nic: Arc<dyn AsyncTun + Sync + Send>) -> Self {
        Connection {
            state: State::Closed(ClosedState::new(
                nic,
                Some(ReceiveSequenceVars::default()),
                Some(SendSequenceVars::default()),
                Arc::new(Mutex::new(RetransmissionQueue::default())),
            )),
        }
    }

    pub async fn on_segment(
        &mut self,
        iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        data: Vec<u8>,
    ) -> TrustResult<Option<ConnectionResult>> {
        let mut ack_received = false;
        if let State::AckRcvd(_) = &self.state {
            ack_received = true;
        }
        if let Some(state) = self.state.on_segment(iph, tcph, data).await? {
            self.state = state.0;
        }
        match &self.state {
            State::Closed(_) => Ok(Some(ConnectionResult::Closed)),
            State::Estab(_) if ack_received => Ok(Some(ConnectionResult::AckReceived)),
            _ => Ok(None),
        }
    }

    pub async fn passive_open(&mut self) -> TrustResult<()> {
        if let Some(state) = self.state.passive_open().await? {
            self.state = state.0;
        }
        Ok(())
    }

    pub async fn open(&mut self, quad: Quad) -> TrustResult<()> {
        if let Some(state) = self.state.open(quad).await? {
            self.state = state.0;
        }
        Ok(())
    }

    pub async fn close(&mut self, quad: Quad) -> TrustResult<()> {
        if let Some(state) = self.state.close(quad).await? {
            self.state = state.0;
        }
        Ok(())
    }

    pub async fn send(&mut self, quad: Quad, data: Vec<u8>) -> TrustResult<()> {
        if let Some(state) = self.state.send(quad, data).await? {
            self.state = state.0;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::IpAddrPort;

    use super::*;
    use std::{
        cmp::Ordering,
        io::{self, Write},
        net::Ipv4Addr,
        str::FromStr,
    };
    struct MockTun {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl MockTun {
        fn new() -> Self {
            MockTun {
                buf: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }
    #[async_trait]
    impl AsyncTun for MockTun {
        async fn read(&self, out: &mut [u8]) -> io::Result<usize> {
            let buf = self.buf.lock().await;
            out.copy_from_slice(&buf);

            Ok(buf.len())
        }

        async fn write(&self, input: &[u8]) -> io::Result<usize> {
            let mut buf = self.buf.lock().await;
            buf.write(input)
        }
    }

    fn quad() -> Quad {
        Quad {
            src: IpAddrPort {
                ip: Ipv4Addr::from_str("192.168.0.1").expect("Failed to parse ip address"),
                port: 1234,
            },
            dst: IpAddrPort {
                ip: Ipv4Addr::from_str("192.168.0.2").expect("Failed to parse ip address"),
                port: 5678,
            },
        }
    }

    #[tokio::test]
    async fn test_mock() {
        let mock_tun = MockTun::new();
        let buf: Vec<u8> = vec![1, 2, 3];
        mock_tun.write(&buf).await.expect("Failed to write bytes");
        let mut buf_out: [u8; 3] = [0; 3];
        let read_bytes = mock_tun
            .read(&mut buf_out)
            .await
            .expect("Failed to read bytes");

        assert_eq!(read_bytes, buf.len());
        assert_eq!(buf.cmp(&buf_out.to_vec()), Ordering::Equal);
    }

    #[tokio::test]
    async fn test_handshake() {
        let mock_tun = Arc::new(MockTun::new());
        let mut conn = Connection::new(mock_tun.clone());
        let quad = quad();
        conn.open(quad).await.expect("Failed to open connection");
        let buf = mock_tun.buf.lock().await;
        crate::debug::debug_packet_print(&buf[4..]);
    }
}
