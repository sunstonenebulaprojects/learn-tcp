use super::serial_numbers::sn_cmp;
use derived::Accessors;
use std::cmp::Ordering;
use tracing::{info, instrument};

#[derive(Debug, Default, Accessors, PartialEq, Eq)]
pub struct ReceiveSequenceVars {
    next: u32,
    window_size: u16,
    up: bool,
    irs: u32,
}

impl ReceiveSequenceVars {
    pub fn init(&mut self) {
        *self = Default::default();
    }
}

impl ReceiveSequenceVars {
    // Segment Length	Receive Window	Test
    //    0	            0	            SEG.SEQ = RCV.NXT
    //    0	            >0	            RCV.NXT =< SEG.SEQ < RCV.NXT+RCV.WND
    //    >0	        0	            not acceptable
    //    >0	        >0	            RCV.NXT =< SEG.SEQ < RCV.NXT+RCV.WND
    //                                  or
    //                                  RCV.NXT =< SEG.SEQ+SEG.LEN-1 < RCV.NXT+RCV.WND
    pub fn incoming_segment_valid(&self, seg_len: u32, seg_sn: u32) -> bool {
        match (seg_len, self.window_size) {
            (0, 0) => seg_sn == self.next(),
            (0, 0..) => self.seg_val_in_range(seg_sn),
            (0.., 0) => false,
            (0.., 0..) => {
                self.seg_val_in_range(seg_sn) || self.seg_val_in_range(seg_sn + (seg_len - 1))
            }
        }
    }

    fn seg_val_in_range(&self, val: u32) -> bool {
        (sn_cmp(self.next(), val) == Ordering::Less || sn_cmp(self.next, val) == Ordering::Equal)
            && sn_cmp(val, self.next() + self.window_size() as u32) == Ordering::Less
    }
}
