#![allow(dead_code)]
use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::states::{ClosedState, State};
use crate::transmission_control_block::{
    ReceiveSequenceVars, RetransmissionQueue, SendSequenceVars,
};
use crate::AsyncTun;
use tracing::{error, info, instrument};

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct LastAck {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Option<ReceiveSequenceVars>,
    send: Option<SendSequenceVars>,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
}

#[async_trait]
impl HandleEvents for LastAck {
    async fn on_segment(
        &mut self,
        _iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        if !tcph.ack {
            error!("We only expect ACK in LastAck state");
        }
        Ok(Some(TransitionState(State::Closed(ClosedState::new(
            self.nic.clone(),
            self.recv.take(),
            self.send.take(),
            self.retransmission_queue.clone(),
        )))))
    }

    async fn passive_open(&mut self) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    async fn open(&mut self, _quad: Quad) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    async fn close(&mut self, _quad: Quad) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    async fn send(&mut self, _quad: Quad, _data: Vec<u8>) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }
}

impl LastAck {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Option<ReceiveSequenceVars>,
        send: Option<SendSequenceVars>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Last ack state");
        assert_ne!(send, None);
        assert_ne!(recv, None);
        Self {
            nic,
            recv,
            send,
            retransmission_queue,
        }
    }
}
