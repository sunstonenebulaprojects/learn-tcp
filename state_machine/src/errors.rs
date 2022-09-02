use thiserror::Error;

pub type TrustResult<T> = Result<T, TrustError>;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrustError {}
