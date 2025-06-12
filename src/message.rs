use crate::data::PlayerSerialize;
use crate::error::TBGError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Message to be sent during lobby phase.
/// Can be sent by and to anyone
/// `T` must be an enum.
#[derive(Serialize, Deserialize, Debug)]
pub enum StartMessage<T> {
    Join(String),
    VoteStart(bool),
    Exit,
    Custom(T),
}

/// Types of messages client will send to server.
/// `T` must be a struct.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage<T> {
    PlayerAction(T),
    /// Player telling server that it's leaving
    Exit,
}
/// Types of messages that client threads will send to main server thread.
/// `T` must be an enum.
#[derive(Debug, Clone)]
pub enum InternalMessage<T> {
    /// Information of a player leaving the game
    PlayerLeft(String),
    /// Tells player that join request is accepted
    AcceptClient(String),
    RejectClient(String),
    /// Server timing out client and forced to lose a die
    TimeOutClient(String),
    /// Tells client that another player has voted
    VoteStart(String, bool),
    /// Server kicking client
    Kick(String),
    Custom(T),
}
/// Types of messages server will send to client threads whom will then
/// propagate it to their clients.
/// `T` must be a struct.
/// `U` must be an enum.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage<T: Clone, U: Clone> {
    SendAll(Vec<PlayerSerialize<T>>),
    /// Information that a player has won
    PlayerWon(String),
    /// Information that it's a new player's turn
    PlayerTurn(String),
    /// Information of a player leaving the game
    PlayerLeft(String),
    /// Information that game started
    StartGame,
    /// Tells player that his name is invalid/taken
    RejectClient(String),
    /// Tells player that join request is accepted
    AcceptClient(String),
    /// Server timing out client and forced to lose a die
    TimeOutClient(String),
    /// Tells client that another player has voted
    VoteStart(String, bool, u8, u8),
    /// Server kicking client
    Kick(String),
    Custom(U),
}

pub fn serialize_message<T: Serialize>(input: &T) -> Vec<u8> {
    let input_string = serde_json::to_string(&input).unwrap();
    let serialized = input_string.as_bytes();
    let length: u32 = serialized.len() as u32;
    let mut vec: Vec<u8> = Vec::with_capacity((length + 4) as usize);
    vec.extend_from_slice(&length.to_le_bytes());
    vec.extend_from_slice(serialized);

    vec
}

pub async fn deserialize_message<T: DeserializeOwned>(
    stream: &mut TcpStream,
) -> Result<Option<T>, TBGError> {
    let mut len_buf: [u8; 4] = [0; 4];
    match stream.read_exact(&mut len_buf).await {
        Ok(0) => return Ok(None),
        Ok(_) => {}
        Err(e) => return Err(TBGError::IoError(e.to_string())),
    }
    let length = u32::from_le_bytes(len_buf) as usize;

    if length > 1024 {
        return Err(TBGError::IoError(format!(
            "Length of message is too large, {length} bytes"
        )));
    }

    let mut buffer = vec![0; length];
    match stream.read_exact(&mut buffer).await {
        Err(e) => return Err(TBGError::IoError(e.to_string())),
        Ok(_) => {
            if buffer.len() < length {
                return Err(TBGError::IoError(format!(
                    "Read {} bytes but expected {} bytes",
                    buffer.len(),
                    length
                )));
            }
        }
    }
    match serde_json::from_slice::<T>(&buffer[..length]) {
        Ok(message) => Ok(Some(message)),
        Err(_) => Err(TBGError::JsonError(String::from(
            "Problem deserializing string",
        ))),
    }
}

pub async fn receive_message<T: for<'de> Deserialize<'de>>(stream: &mut TcpStream) -> Option<T> {
    let result = deserialize_message(stream).await;
    result.unwrap_or_else(|_| None)
}

pub async fn receive_message_with_timeout<T: for<'de> Deserialize<'de>>(
    stream: &mut TcpStream,
    duration: Duration,
) -> Option<T> {
    let timeout_result = timeout(duration, deserialize_message(stream)).await;
    match timeout_result {
        Ok(x) => x.unwrap_or_else(|_| None),
        Err(_) => None,
    }
}
