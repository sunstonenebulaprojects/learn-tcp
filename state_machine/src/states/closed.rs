#![allow(dead_code)]
use super::syn_sent::SynSentState;
use super::State;
use crate::connection::{HandleEvents, TransitionState};
use crate::quad::Quad;
use crate::transmission_control_block::{
    ReceiveSequenceVars, RetransmissionQueue, SendSequenceVars,
};
use crate::{send, AsyncTun};
use tracing::{info, instrument};

use std::sync::Arc;
use tokio::sync::Mutex;

use super::ListenState;
use crate::errors::TrustResult;
use async_trait::async_trait;

pub struct ClosedState {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Option<ReceiveSequenceVars>,
    send: Option<SendSequenceVars>,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
}

#[async_trait]
impl HandleEvents for ClosedState {
    async fn on_segment(
        &mut self,
        _iph: etherparse::Ipv4Header,
        _tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    #[instrument(skip(self))]
    async fn passive_open(&mut self) -> TrustResult<Option<TransitionState>> {
        self.send.as_mut().unwrap().init();

        Ok(Some(TransitionState(State::Listen(ListenState::new(
            self.nic.clone(),
            self.recv.take(),
            self.send.take(),
            self.retransmission_queue.clone(),
        )))))
    }

    #[instrument(skip_all)]
    async fn open(&mut self, quad: Quad) -> TrustResult<Option<TransitionState>> {
        self.send.as_mut().unwrap().init();

        send::send_syn(
            self.nic.clone(),
            quad.dst,
            quad.src,
            self.send.as_ref().unwrap().iss(),
            1000,
        )
        .await;

        Ok(Some(TransitionState(State::SynSent(SynSentState::new(
            self.nic.clone(),
            self.recv.take(),
            self.send.take(),
            self.retransmission_queue.clone(),
        )))))
    }

    async fn close(&mut self, _quad: Quad) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }
    async fn send(&mut self, _quad: Quad, _data: Vec<u8>) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }
}

impl ClosedState {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Option<ReceiveSequenceVars>,
        send: Option<SendSequenceVars>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Closed state");
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
