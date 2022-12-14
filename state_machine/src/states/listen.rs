#![allow(dead_code)]
use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::send;
use crate::states::syn_received::SynReceivedState;
use crate::states::State;
use crate::transmission_control_block::{
    ReceiveSequenceVars, RetransmissionQueue, SendSequenceVars,
};
use crate::{debug, AsyncTun};

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, instrument};

pub struct ListenState {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Option<ReceiveSequenceVars>,
    send: Option<SendSequenceVars>,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
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
        &mut self,
        iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        debug::print_info(&iph, &tcph, &data);

        if !tcph.syn {
            error!("We expect only SYN packet in Listen state");
            return Ok(None);
        }

        let recv = self.recv.as_mut().unwrap();
        let send = self.send.as_mut().unwrap();

        recv.set_window_size(tcph.window_size)
            .set_irs(tcph.sequence_number)
            .set_next(tcph.sequence_number.wrapping_add(1));

        let iss = send.iss();
        send::send_syn_ack(self.nic.clone(), iph, tcph.clone(), send.iss(), recv.next()).await;

        send.set_window_size(tcph.window_size)
            .set_next(iss.wrapping_add(1))
            .set_una(iss);

        Ok(Some(TransitionState(State::SynRcvd(
            SynReceivedState::new(
                self.nic.clone(),
                self.recv.take(),
                self.send.take(),
                self.retransmission_queue.clone(),
            ),
        ))))
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

impl ListenState {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Option<ReceiveSequenceVars>,
        send: Option<SendSequenceVars>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Listen state");
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
