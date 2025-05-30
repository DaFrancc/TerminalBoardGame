use crossterm::terminal::SetSize;
use serde::Deserialize;
use std::{f32::consts::E, fmt, io};

// #[derive(Debug, Deserialize)]
// pub enum TBGError {
//     IoError(String),
//     JsonError(String),
//     TimeoutError,
//     InvalidName,
//     FailToConnect(String),
//     FailToInitializeServer(String),
//     LockPoisoned,
//     Other(String),
// }

// impl fmt::Display for TBGError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             TBGError::IoError(e) => write!(f, "IO Error: {}", e),
//             TBGError::JsonError(e) => write!(f, "JSON Error: {}", e),
//             TBGError::TimeoutError => write!(f, "Timeout occurred"),
//             TBGError::InvalidName => write!(f, "Name is already taken or otherwise invalid"),
//             TBGError::FailToConnect(e) => write!(f, "Fail to connect to server: {}", e),
//             TBGError::FailToInitializeServer(e) => write!(f, "Fail to initialize server: {}", e),
//             TBGError::LockPoisoned => write!(f, "Lock poisoned"),
//             TBGError::Other(msg) => write!(f, "{}", msg),
//         }
//     }
// }

// impl std::error::Error for TBGError {}

// impl From<io::Error> for TBGError {
//     fn from(e: io::Error) -> Self {
//         TBGError::IoError(e.to_string())
//     }
// }

// impl From<serde_json::Error> for TBGError {
//     fn from(e: serde_json::Error) -> Self {
//         TBGError::JsonError(e.to_string())
//     }
// }

#[derive(Debug, Deserialize)]
pub enum TBGError {
    /* Shared */
    FailSerialize,
    TimeOut,
    InvalidID,

    /* Server */
    InvalidName,
    SendFailure,
    FailToConnect(String),

    /* Message Passing */
    DataStreamEarlyEOF,
    MessageOverflow(usize),
    SizeMismatch(usize, usize),
    FailDeserialize,
    EmptyMessage,

    /* Player List */
    DuplicateEntry,
}

impl fmt::Display for TBGError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TBGError::FailSerialize => write!(f, "Failure to serialize string."),
            TBGError::TimeOut => write!(f, "No message received before timeout ended."),
            TBGError::InvalidID => write!(f, "No pair found for given ID."),
            TBGError::InvalidName => write!(f, "Name is already taken or otherwise invalid"),
            TBGError::SendFailure => write!(f, "Failed to send message to client."),
            TBGError::FailToConnect(ip) => write!(f, "IP {ip} failed to connect to server."),
            TBGError::DataStreamEarlyEOF => write!(f, "Data stream ended early"),
            TBGError::MessageOverflow(size) => {
                write!(f, "Incoming message size of {size} bytes too large")
            }
            TBGError::SizeMismatch(read, expected) => {
                write!(f, "Read {read} bytes but expected {expected} bytes.")
            },
            TBGError::FailDeserialize => write!(f, "Failure to deserialize message."),
            TBGError::EmptyMessage => write!(f, "Message is empty."),
            TBGError::DuplicateEntry => {
                write!(f, "Name is already taken or otherwise invalid.")
            }
        }
    }
}

impl std::error::Error for TBGError {}
