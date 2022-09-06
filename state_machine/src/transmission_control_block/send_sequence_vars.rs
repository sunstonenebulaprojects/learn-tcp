use std::cmp::Ordering;

use super::serial_numbers::sn_cmp;
use derived::Accessors;
use tracing::{info, instrument};

const ONE_SECOND_IN_MICROSECONDS: f64 = 1_000_000.0;

#[derive(Debug, Default, Accessors, PartialEq)]
pub struct SendSequenceVars {
    una: u32,
    next: u32,
    window_size: u16,
    up: bool,
    wl1: usize,
    wl2: usize,
    iss: u32,
    rto: f64,
    srtt: f64,
    rttvar: f64,
}

impl SendSequenceVars {
    #[instrument(skip(self))]
    pub fn init(&mut self) {
        info!("Initialize send sequence vars");
        // TODO: init initial sequence number (iss)
        *self = Default::default();
        self.rto = ONE_SECOND_IN_MICROSECONDS;
    }
}

impl SendSequenceVars {
    /// check SND.UNA < SEG.ACK <= SND.NXT
    pub fn acknowledgment_number_valid(&self, acknowledgment_number: u32) -> bool {
        sn_cmp(self.una(), acknowledgment_number) == Ordering::Less
            && (sn_cmp(acknowledgment_number, self.next()) == Ordering::Less
                || sn_cmp(acknowledgment_number, self.next()) == Ordering::Equal)
    }

    // From Rfc 6298:
    // When the first RTT measurement R is made, the host MUST set
    //      SRTT <- R
    //      RTTVAR <- R/2
    //      RTO <- SRTT + max (G, K*RTTVAR)
    //           where K = 4.
    // When a subsequent RTT measurement R’ is made, a host MUST set
    //      RTTVAR <- (1 - beta) * RTTVAR + beta * |SRTT - R’|
    //      SRTT <- (1 - alpha) * SRTT + alpha * R’
    //           where alpha=1/8 and beta=1/4
    pub fn update_rto(&mut self, rtt: f64) {
        // first time update
        const K: f64 = 4.0;
        const ALPHA: f64 = 1.0 / 8.0;
        const BETA: f64 = 1.0 / 4.0;
        if self.rto == ONE_SECOND_IN_MICROSECONDS {
            self.srtt = rtt;
            self.rttvar = rtt / 2.0;
            self.rto = self.srtt + 1f64.max(K * self.rttvar);
        } else {
            self.rttvar = (1.0 - BETA) * self.rttvar + BETA * (self.srtt - rtt).abs();
            self.srtt = (1.0 - ALPHA) * self.srtt + ALPHA * rtt;
            self.rto = self.srtt + 1f64.max(K * self.rttvar);
        }
    }
}
