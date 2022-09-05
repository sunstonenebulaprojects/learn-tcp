use std::collections::vec_deque::VecDeque;
use std::time::Instant;

use crate::IpAddrPort;

pub struct Segment {
    pub buffer: Vec<u8>,
    pub sequence_number: u32,
    pub retransmissions: u32,
    pub sent: Instant,
    pub to: IpAddrPort,
    pub from: IpAddrPort,
}

impl Segment {
    pub fn new(buffer: Vec<u8>, sequence_number: u32, to: IpAddrPort, from: IpAddrPort) -> Self {
        Self {
            buffer,
            sequence_number,
            retransmissions: 0,
            sent: Instant::now(),
            to,
            from,
        }
    }
}
/// Segments in the queue are implicetly sorted by SN.
/// Other side cannot ACK segments out of order:
///    if 1, 2, 3, 4 segments are sent, 1, 3, 4 are received
///    we'll get ACK for 1 multiple times until lost packet 2 will be retransmitted and received
/// TODO: functionality to remove multiple segments <= ACK number
#[derive(Default)]
pub struct RetransmissionQueue {
    queue: VecDeque<Segment>,
}

impl RetransmissionQueue {
    pub fn add(&mut self, segment: Segment) {
        self.queue.push_back(segment);
    }

    pub fn pop(&mut self) {
        assert!(!self.queue.is_empty());

        self.queue.pop_front();
    }

    pub fn front(&mut self) -> &mut Segment {
        self.queue.front_mut().unwrap()
    }

    pub fn restransmission_count(&self) -> u32 {
        self.queue.front().as_ref().unwrap().retransmissions
    }
}
