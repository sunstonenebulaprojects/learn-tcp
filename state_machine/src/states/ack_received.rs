#![allow(dead_code)]
use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::transmission_control_block::ReceiveSequenceVars;
use crate::transmission_control_block::SendSequenceVars;
use crate::AsyncTun;
use tracing::{error, info, instrument};

use async_trait::async_trait;
use std::borrow::BorrowMut;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use super::established::EstablishedState;
use super::State;

pub struct AckReceivedState {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Arc<Mutex<ReceiveSequenceVars>>,
    send: Arc<Mutex<SendSequenceVars>>,
    start: Instant,
}

#[async_trait]
impl HandleEvents for AckReceivedState {
    async fn on_segment(
        &self,
        _iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        if !tcph.ack {
            error!("We only expect ACK in AckReceived state");
            return Ok(None);
        }

        let mut send_guard = self.send.as_ref().lock().await;
        let send = send_guard.borrow_mut();

        // check send.una < ACK num <= send.next
        if !send.acknowledgment_number_valid(tcph.acknowledgment_number) {
            error!(
                "Check [send.una < ACK num <= send.next] failed, una: {}, ack: {}, next: {}",
                send.una(),
                tcph.acknowledgment_number,
                send.next()
            );
            return Ok(None);
        }
        info!(
            micros = self.start.elapsed().as_micros(),
            "RTT sample on ACK"
        );
        send.set_una(tcph.acknowledgment_number);

        Ok(Some(TransitionState(State::Estab(EstablishedState::new(
            self.nic.clone(),
            self.recv.clone(),
            self.send.clone(),
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

impl AckReceivedState {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Arc<Mutex<ReceiveSequenceVars>>,
        send: Arc<Mutex<SendSequenceVars>>,
    ) -> Self {
        info!("Transitioned to Ack received state");
        Self {
            nic,
            recv,
            send,
            start: Instant::now(),
        }
    }
}
