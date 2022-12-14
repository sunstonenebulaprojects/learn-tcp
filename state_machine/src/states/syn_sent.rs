use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::transmission_control_block::{
    ReceiveSequenceVars, RetransmissionQueue, SendSequenceVars,
};
use crate::{send, AsyncTun};
use tracing::{error, info, instrument};

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::established::EstablishedState;
use super::State;

pub struct SynSentState {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Option<ReceiveSequenceVars>,
    send: Option<SendSequenceVars>,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
}

///  From RFC 9293
///
///     TCP Peer A                                           TCP Peer B
///
///     1.  CLOSED                                               LISTEN
///     2.  SYN-SENT    --> <SEQ=100><CTL=SYN>               --> SYN-RECEIVED
///     3.  ESTABLISHED <-- <SEQ=300><ACK=101><CTL=SYN,ACK>  <-- SYN-RECEIVED
///     4.  ESTABLISHED --> <SEQ=101><ACK=301><CTL=ACK>       --> ESTABLISHED
///     5.  ESTABLISHED --> <SEQ=101><ACK=301><CTL=ACK><DATA> --> ESTABLISHED
#[async_trait]
impl HandleEvents for SynSentState {
    async fn on_segment(
        &mut self,
        iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        if !(tcph.syn && tcph.ack) {
            error!("We only expect SYN,ACK in SynSent state");
            return Ok(None);
        }

        let recv = self.recv.as_mut().unwrap();
        let send = self.send.as_mut().unwrap();

        recv.set_window_size(tcph.window_size)
            .set_irs(tcph.sequence_number)
            .set_next(tcph.sequence_number.wrapping_add(1));

        let iss = send.iss();
        send.set_window_size(tcph.window_size)
            .set_next(iss.wrapping_add(1))
            .set_una(iss);

        send::send_ack(
            self.nic.clone(),
            iph,
            tcph.clone(),
            send.next(),
            recv.next(),
        )
        .await;

        Ok(Some(TransitionState(State::Estab(EstablishedState::new(
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

impl SynSentState {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Option<ReceiveSequenceVars>,
        send: Option<SendSequenceVars>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Syn sent state");
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
