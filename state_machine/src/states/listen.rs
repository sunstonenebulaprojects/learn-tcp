#![allow(dead_code)]
use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::send;
use crate::states::syn_received::SynReceivedState;
use crate::states::State;
use crate::transmission_control_block::ReceiveSequenceVars;
use crate::transmission_control_block::SendSequenceVars;
use crate::{debug, AsyncTun};

use async_trait::async_trait;
use std::borrow::BorrowMut;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, instrument};

pub struct ListenState {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Arc<Mutex<ReceiveSequenceVars>>,
    send: Arc<Mutex<SendSequenceVars>>,
}

/// Guided by RFC 9293 3.10.7.2. LISTEN STATE
///
/// From this state we transition to SYN-RECEIVED state
/// --> syn received
/// <-- syn,ack sent
/// go to SynRcvd state and wait for ack
#[async_trait]
impl HandleEvents for ListenState {
    #[instrument(skip_all)]
    async fn on_segment(
        &self,
        iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        debug::print_info(&iph, &tcph, &data);

        if !tcph.syn {
            error!("We expect only SYN packet in Listen state");
            return Ok(None);
        }

        let mut recv_guard = self.recv.as_ref().lock().await;
        let recv = recv_guard.borrow_mut();
        let mut send_guard = self.send.as_ref().lock().await;
        let send = send_guard.borrow_mut();

        recv.set_window_size(tcph.window_size)
            .set_irs(tcph.sequence_number)
            .set_next(tcph.sequence_number.wrapping_add(1));

        let iss = send.iss();
        send::send_syn_ack(self.nic.clone(), iph, tcph.clone(), send.iss(), recv.next()).await;

        send.set_window_size(tcph.window_size)
            .set_next(iss.wrapping_add(1))
            .set_una(iss);

        Ok(Some(TransitionState(State::SynRcvd(
            SynReceivedState::new(self.nic.clone(), self.recv.clone(), self.send.clone()),
        ))))
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

impl ListenState {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Arc<Mutex<ReceiveSequenceVars>>,
        send: Arc<Mutex<SendSequenceVars>>,
    ) -> Self {
        info!("Transitioned to Listen state");
        Self { nic, recv, send }
    }
}
