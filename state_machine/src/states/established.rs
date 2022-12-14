#![allow(dead_code)]
use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::states::fin_wait1::FinWait1State;
use crate::states::last_ack::LastAck;
use crate::states::State;
use crate::transmission_control_block::{
    ReceiveSequenceVars, RetransmissionQueue, Segment, SendSequenceVars,
};
use crate::{send, AsyncTun};
use tracing::{error, info, instrument};

use async_trait::async_trait;
use std::borrow::BorrowMut;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::ack_received::AckReceivedState;

pub struct EstablishedState {
    nic: Arc<dyn AsyncTun + Sync + Send>,
    recv: Option<ReceiveSequenceVars>,
    send: Option<SendSequenceVars>,
    retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
}

#[async_trait]
impl HandleEvents for EstablishedState {
    async fn on_segment(
        &mut self,
        iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
        data: Vec<u8>,
    ) -> TrustResult<Option<TransitionState>> {
        if tcph.fin {
            return self.handle_fin(iph, tcph).await;
        }

        let recv = self.recv.as_mut().unwrap();
        let send = self.send.as_ref().unwrap();

        if !recv.incoming_segment_valid(data.len() as u32, tcph.sequence_number) {
            error!(
                "Data already received, sequence number: {}, expected: {}",
                tcph.sequence_number,
                recv.next()
            );
            return Ok(None);
        }

        recv.set_next(tcph.sequence_number.wrapping_add(data.len() as u32));

        send::send_ack(
            self.nic.clone(),
            iph.clone(),
            tcph.clone(),
            send.next(),
            recv.next(),
        )
        .await;

        Ok(None)
    }

    async fn passive_open(&mut self) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    async fn open(&mut self, _quad: Quad) -> TrustResult<Option<TransitionState>> {
        unreachable!()
    }

    async fn close(&mut self, quad: Quad) -> TrustResult<Option<TransitionState>> {
        info!("CLOSE call rceived");

        let recv = self.recv.as_ref().unwrap();
        let send = self.send.as_mut().unwrap();

        let ack = recv.next();

        let send_next = send.next();
        send.set_next(send_next.wrapping_add(1));

        let send_una = send.una();
        let send_window_size = send.window_size();
        send::send_fin(
            self.nic.clone(),
            quad.dst,
            quad.src,
            send_una,
            ack,
            send_window_size,
        )
        .await;

        Ok(Some(TransitionState(State::FinWait(FinWait1State::new(
            self.nic.clone(),
            self.recv.take(),
            self.send.take(),
            self.retransmission_queue.clone(),
        )))))
    }

    async fn send(&mut self, quad: Quad, data: Vec<u8>) -> TrustResult<Option<TransitionState>> {
        info!("SEND call received");

        let send = self.send.as_mut().unwrap();

        let send_next = send.next();
        send.set_una(send_next);
        let send_una = send.una();
        send.set_next(send_una.wrapping_add(data.len() as u32));

        send::send_data(
            self.nic.clone(),
            quad.dst,
            quad.src,
            send_una,
            self.recv.as_ref().unwrap().next(),
            send.window_size(),
            &data,
        )
        .await;
        {
            let mut retransmission_queue_guard = self.retransmission_queue.as_ref().lock().await;
            let retransmission_queue = retransmission_queue_guard.borrow_mut();
            retransmission_queue.add(Segment::new(data, send_una, quad.dst, quad.src));
        }

        Ok(Some(TransitionState(State::AckRcvd(
            AckReceivedState::new(
                self.nic.clone(),
                self.recv.take(),
                self.send.take(),
                self.retransmission_queue.clone(),
            )
            .await,
        ))))
    }
}

impl EstablishedState {
    #[instrument(skip_all)]
    pub fn new(
        nic: Arc<dyn AsyncTun + Sync + Send>,
        recv: Option<ReceiveSequenceVars>,
        send: Option<SendSequenceVars>,
        retransmission_queue: Arc<Mutex<RetransmissionQueue>>,
    ) -> Self {
        info!("Transitioned to Established state");
        assert_ne!(send, None);
        assert_ne!(recv, None);
        Self {
            nic,
            recv,
            send,
            retransmission_queue,
        }
    }
    async fn handle_fin(
        &mut self,
        iph: etherparse::Ipv4Header,
        tcph: etherparse::TcpHeader,
    ) -> TrustResult<Option<TransitionState>> {
        let recv = self.recv.as_mut().unwrap();
        let send = self.send.as_ref().unwrap();

        let send_next = send.next();

        recv.set_next(tcph.sequence_number.wrapping_add(1));
        send::send_ack(
            self.nic.clone(),
            iph.clone(),
            tcph.clone(),
            send_next,
            recv.next(),
        )
        .await;
        send::send_fin_ack(self.nic.clone(), &iph, &tcph, send_next, recv.next()).await;
        Ok(Some(TransitionState(State::LastAck(LastAck::new(
            self.nic.clone(),
            self.recv.take(),
            self.send.take(),
            self.retransmission_queue.clone(),
        )))))
    }
}
