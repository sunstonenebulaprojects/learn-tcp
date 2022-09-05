#![allow(dead_code)]
use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::transmission_control_block::{
    ReceiveSequenceVars, RetransmissionQueue, SendSequenceVars,
};
use crate::AsyncTun;
use tracing::{error, info, info_span, instrument};
use tracing_futures::{Instrument, Instrumented};

use crate::send;
use async_trait::async_trait;
use std::borrow::BorrowMut;
use std::sync::Arc;
use std::time::Instant;
use tokio::{sync::Mutex, task::JoinHandle};

use super::established::EstablishedState;
use super::State;

pub struct AckReceivedState {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Arc<Mutex<ReceiveSequenceVars>>,
    send: Arc<Mutex<SendSequenceVars>>,
    rto_sample: Instant,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    retransmission_timer: Instrumented<JoinHandle<()>>,
}

#[async_trait]
impl HandleEvents for AckReceivedState {
    async fn on_segment(
        &mut self,
        _iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        _data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        if !tcph.ack {
            error!("We only expect ACK in AckReceived state");
            return Ok(None);
        }

        {
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
            send.set_una(tcph.acknowledgment_number);
        }

        self.process_acked_segment(tcph.acknowledgment_number).await;

        Ok(Some(TransitionState(State::Estab(EstablishedState::new(
            self.nic.clone(),
            self.recv.clone(),
            self.send.clone(),
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

impl AckReceivedState {
    #[instrument(skip_all)]
    pub async fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Arc<Mutex<ReceiveSequenceVars>>,
        send: Arc<Mutex<SendSequenceVars>>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Ack received state");
        let rto = { send.as_ref().lock().await.rto() };
        let rqueue = retransmission_queue.clone();
        let nic_cloned = nic.clone();
        let (ack, window_size) = {
            let recv = recv.as_ref().lock().await;
            (recv.next(), recv.window_size())
        };
        let handle = tokio::task::Builder::new()
            .name("retransmit_segment")
            .spawn(async move {
                info!(ms = rto.ceil(), "Sleeping in timeout future");
                tokio::time::sleep(std::time::Duration::from_micros(rto.ceil() as u64)).await;

                let mut rqueue = rqueue.as_ref().lock().await;
                let rqueue = rqueue.borrow_mut();
                let segment = rqueue.front();
                segment.retransmissions += 1;
                info!(
                    sn = segment.sequence_number,
                    attempt = segment.retransmissions,
                    "Retransmit segment"
                );
                send::send_data(
                    nic_cloned,
                    segment.to,
                    segment.from,
                    segment.sequence_number,
                    ack,
                    window_size,
                    &segment.buffer,
                )
                .await;
            })
            .instrument(info_span!("Retransmission"));
        Self {
            nic,
            recv,
            send,
            rto_sample: Instant::now(),
            retransmission_queue,
            retransmission_timer: handle,
        }
    }

    #[instrument(skip_all)]
    async fn process_acked_segment(&self, acknowledgment_number: u32) {
        let mut send_guard = self.send.as_ref().lock().await;
        let send = send_guard.borrow_mut();
        info!("Locking queue");
        let mut retransmission_queue_guard = self.retransmission_queue.as_ref().lock().await;
        let retransmission_queue = retransmission_queue_guard.borrow_mut();

        let segment = retransmission_queue.front();
        assert_eq!(
            segment.sequence_number + segment.buffer.len() as u32,
            acknowledgment_number
        );
        info!(
            micros = segment.sent.elapsed().as_micros(),
            "RTT sample on ACK"
        );
        info!(
            sn = segment.sequence_number,
            "Popping segment from retransmission queue"
        );
        if segment.retransmissions == 0 {
            send.update_rto(segment.sent.elapsed().as_micros() as f64);
        }
        retransmission_queue.pop();
        for _ in 0..3 {
            self.retransmission_timer.inner().abort();
            if self.retransmission_timer.inner().is_finished() {
                info!("Retransmission timer aborted");
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        info!(
            aborted = self.retransmission_timer.inner().is_finished(),
            "Retransmission timer aborted"
        );
    }
}

impl Drop for AckReceivedState {
    fn drop(&mut self) {
        if !self.retransmission_timer.inner().is_finished() {
            self.retransmission_timer.inner().abort();
        }
    }
}
