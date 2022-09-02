mod ack_received;
mod closed;
mod established;
mod fin_wait1;
mod last_ack;
mod listen;
mod syn_received;
mod syn_sent;

use crate::connection::{HandleEvents, TransitionState};
use crate::errors::TrustResult;
use crate::quad::Quad;
use crate::states::ack_received::AckReceivedState;
pub use crate::states::closed::ClosedState;
use crate::states::established::EstablishedState;
use crate::states::fin_wait1::FinWait1State;
use crate::states::last_ack::LastAck;
pub use crate::states::listen::ListenState;
use crate::states::syn_received::SynReceivedState;
use crate::states::syn_sent::SynSentState;
use derived::HandleEvents;

use async_trait::async_trait;

#[derive(HandleEvents)]
pub enum State {
    Closed(ClosedState),
    Listen(ListenState),
    SynSent(SynSentState),
    SynRcvd(SynReceivedState),
    Estab(EstablishedState),
    LastAck(LastAck),
    AckRcvd(AckReceivedState),
    FinWait(FinWait1State),
}
