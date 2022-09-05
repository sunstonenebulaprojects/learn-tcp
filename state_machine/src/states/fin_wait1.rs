#![allow(dead_code)]
use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::states::{ClosedState, State};
use crate::transmission_control_block::{
    ReceiveSequenceVars, RetransmissionQueue, SendSequenceVars,
};
use crate::{send, AsyncTun};
use tracing::{info, instrument};

use async_trait::async_trait;
use std::borrow::BorrowMut;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct FinWait1State {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Arc<Mutex<ReceiveSequenceVars>>,
    send: Arc<Mutex<SendSequenceVars>>,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
}

#[async_trait]
impl HandleEvents for FinWait1State {
    async fn on_segment(
        &self,
        iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        let mut recv_guard = self.recv.as_ref().lock().await;
        let recv = recv_guard.borrow_mut();
        let send = self.send.as_ref().lock().await;

        if tcph.ack && tcph.fin {
            recv.set_next(tcph.sequence_number.wrapping_add(1));
            send::send_ack(self.nic.clone(), iph, tcph, send.next(), recv.next()).await;
            Ok(Some(TransitionState(State::Closed(ClosedState::new(
                self.nic.clone(),
                self.recv.clone(),
                self.send.clone(),
                self.retransmission_queue.clone(),
            )))))
        } else {
            Ok(None)
        }
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

impl FinWait1State {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Arc<Mutex<ReceiveSequenceVars>>,
        send: Arc<Mutex<SendSequenceVars>>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Fin-wait1 state");
        Self {
            nic,
            recv,
            send,
            retransmission_queue,
        }
    }
}
