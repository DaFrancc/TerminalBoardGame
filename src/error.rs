use crossterm::terminal::SetSize;
use serde::Deserialize;
use std::{f32::consts::E, fmt, io};

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

/* Message passing errors */

#[derive(Debug, Deserialize)]
pub enum MessageError {
    FailSerialize,
    DataStreamEarlyEOF,
    MessageOverflow(usize),
    SizeMismatch(usize, usize),
    FailDeserialize,
    TimedOut

}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageError::FailSerialize => write!(f, "Failure to serialize string"),
            MessageError::DataStreamEarlyEOF => write!(f, "Data stream ended early"),
            MessageError::MessageOverflow(size) => {
                write!(f, "Incoming message size of {size} bytes too large")
            }
            MessageError::SizeMismatch(read, expected) => {
                write!(f, "Read {read} bytes but expected {expected} bytes.")
            },
            MessageError::FailDeserialize => write!(f, "Failure to deserialize message."),
            MessageError::TimedOut => write!(f, "No message received before timeout ended.")
        }
    }
}

impl std::error::Error for MessageError {}

/* Player List errors */

#[derive(Debug)]

pub enum PlayerListError {
    DuplicateEntry,
    InvalidID,
}

impl fmt::Display for PlayerListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerListError::DuplicateEntry => {
                write!(f, "Name is already taken or otherwise invalid.")
            }
            PlayerListError::InvalidID => write!(f, "No pair found for given ID."),
        }
    }
}

impl std::error::Error for PlayerListError {}
