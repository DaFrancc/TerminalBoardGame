use std::{fmt, io};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum TBGError {
    IoError(String),
    JsonError(String),
    TimeoutError,
    InvalidName,
    FailToConnect(String),
    FailToInitializeServer(String),
    LockPoisoned,
    Other(String),
}

impl fmt::Display for TBGError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TBGError::IoError(e) => write!(f, "IO Error: {}", e),
            TBGError::JsonError(e) => write!(f, "JSON Error: {}", e),
            TBGError::TimeoutError => write!(f, "Timeout occurred"),
            TBGError::InvalidName => write!(f, "Name is already taken or otherwise invalid"),
            TBGError::FailToConnect(e) => write!(f, "Fail to connect to server: {}", e),
            TBGError::FailToInitializeServer(e) => write!(f, "Fail to initialize server: {}", e),
            TBGError::LockPoisoned => write!(f, "Lock poisoned"),
            TBGError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for TBGError {}

impl From<io::Error> for TBGError {
    fn from(e: io::Error) -> Self {
        TBGError::IoError(e.to_string())
    }
}

impl From<serde_json::Error> for TBGError {
    fn from(e: serde_json::Error) -> Self {
        TBGError::JsonError(e.to_string())
    }
}