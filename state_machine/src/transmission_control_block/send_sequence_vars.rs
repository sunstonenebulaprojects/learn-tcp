use std::cmp::Ordering;

use super::serial_numbers::sn_cmp;
use derived::Accessors;
use tracing::{info, instrument};

#[derive(Debug, Default, Accessors)]
pub struct SendSequenceVars {
    una: u32,
    next: u32,
    window_size: u16,
    up: bool,
    wl1: usize,
    wl2: usize,
    iss: u32,
    rto: u32,
    srtt: u32,
    rttvar: u32,
}

impl SendSequenceVars {
    #[instrument(skip(self))]
    pub fn init(&mut self) {
        info!("Initialize send sequence vars");
        // TODO: init initial sequence number (iss)
        *self = Default::default();
    }
}

impl SendSequenceVars {
    /// check SND.UNA < SEG.ACK <= SND.NXT
    pub fn acknowledgment_number_valid(&self, acknowledgment_number: u32) -> bool {
        sn_cmp(self.una(), acknowledgment_number) == Ordering::Less
            && (sn_cmp(acknowledgment_number, self.next()) == Ordering::Less
                || sn_cmp(acknowledgment_number, self.next()) == Ordering::Equal)
    }
}
