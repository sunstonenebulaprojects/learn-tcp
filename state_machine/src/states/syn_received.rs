#![allow(dead_code)]
use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::transmission_control_block::ReceiveSequenceVars;
use crate::transmission_control_block::SendSequenceVars;
use crate::AsyncTun;
use tracing::{error, info, instrument};

use std::sync::Arc;
use tokio::sync::Mutex;

use super::established::EstablishedState;
use super::State;
use async_trait::async_trait;

pub struct SynReceivedState {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Arc<Mutex<ReceiveSequenceVars>>,
    send: Arc<Mutex<SendSequenceVars>>,
}

#[async_trait]
impl HandleEvents for SynReceivedState {
    async fn on_segment(
        &self,
        _iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        if !tcph.ack {
            error!("We expect only ACK packet in SYN_RECEIVED state");
            return Ok(None);
        }

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

impl SynReceivedState {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Arc<Mutex<ReceiveSequenceVars>>,
        send: Arc<Mutex<SendSequenceVars>>,
    ) -> Self {
        info!("Transitioned to Syn received state");
        Self { nic, recv, send }
    }
}