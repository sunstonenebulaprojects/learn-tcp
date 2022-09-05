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
    recv: Arc<Mutex<ReceiveSequenceVars>>,
    send: Arc<Mutex<SendSequenceVars>>,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
}

#[async_trait]
impl HandleEvents for ClosedState {
    async fn on_segment(
        &self,
        _iph: etherparse::Ipv4Header,
        _tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    #[instrument(skip(self))]
    async fn passive_open(&self) -> TrustResult<Option<TransitionState>> {
        self.send.as_ref().lock().await.init();

        Ok(Some(TransitionState(State::Listen(ListenState::new(
            self.nic.clone(),
            self.recv.clone(),
            self.send.clone(),
            self.retransmission_queue.clone(),
        )))))
    }

    #[instrument(skip_all)]
    async fn open(&self, quad: Quad) -> TrustResult<Option<TransitionState>> {
        self.send.as_ref().lock().await.init();

        send::send_syn(
            self.nic.clone(),
            quad.dst,
            quad.src,
            self.send.as_ref().lock().await.iss(),
            1000,
        )
        .await;

        Ok(Some(TransitionState(State::SynSent(SynSentState::new(
            self.nic.clone(),
            self.recv.clone(),
            self.send.clone(),
            self.retransmission_queue.clone(),
        )))))
    }

    async fn close(&self, _quad: Quad) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }
    async fn send(&self, _quad: Quad, _data: Vec<u8>) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }
}

impl ClosedState {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Arc<Mutex<ReceiveSequenceVars>>,
        send: Arc<Mutex<SendSequenceVars>>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Closed state");
        Self {
            nic,
            recv,
            send,
            retransmission_queue,
        }
    }
}
