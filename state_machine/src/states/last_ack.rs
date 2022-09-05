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
    recv: Arc<Mutex<ReceiveSequenceVars>>,
    send: Arc<Mutex<SendSequenceVars>>,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
}

#[async_trait]
impl HandleEvents for LastAck {
    async fn on_segment(
        &self,
        _iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        if !tcph.ack {
            error!("We only expect ACK in LastAck state");
        }
        Ok(Some(TransitionState(State::Closed(ClosedState::new(
            self.nic.clone(),
            self.recv.clone(),
            self.send.clone(),
            self.retransmission_queue.clone(),
        )))))
    }

    async fn passive_open(&self) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    async fn open(&self, _quad: Quad) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    async fn close(&self, _quad: Quad) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    async fn send(&self, _quad: Quad, _data: Vec<u8>) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }
}

impl LastAck {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Arc<Mutex<ReceiveSequenceVars>>,
        send: Arc<Mutex<SendSequenceVars>>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Last ack state");
        Self {
            nic,
            recv,
            send,
            retransmission_queue,
        }
    }
}
