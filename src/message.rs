use crate::data::PlayerSerialize;
use crate::error::TBGError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

#[repr(u8)]
pub enum WaitTimeSec {
    Quick = 1,
    Short = 5,
    Med = 10,
    Long = 30
}

/// Message to be sent during lobby phase.
/// Can be sent internally or to clients.
/// `T` must be an enum.
#[derive(Serialize, Deserialize, Debug)]
pub enum ClientStartMessage<T> {
    Join(String),
    VoteStart(bool),
    Exit,
    Custom(T),
}
/// Message to be sent during lobby phase.
/// Sent by clients to client threads.
/// `T` must be an enum.
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerStartMessage<T> {
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

const MAX_MESSAGE_LENGTH: usize = 1024;

pub fn serialize_message<T: Serialize>(input: &T) -> Result<Vec<u8>, TBGError> {
    let input_string = match serde_json::to_string(&input) {
        Ok(s) => s,
        Err(_) => return Err(TBGError::FailSerialize),
    };
    let serialized = input_string.as_bytes();
    let length: u32 = serialized.len() as u32;
    let mut vec: Vec<u8> = Vec::with_capacity((length + 4) as usize);
    vec.extend_from_slice(&length.to_le_bytes());
    vec.extend_from_slice(serialized);

    Ok(vec)
}

pub async fn deserialize_message<T: DeserializeOwned>(
    stream: &mut TcpStream,
) -> Result<T, TBGError> {
    let mut len_buf: [u8; 4] = [0; 4];
    if let Err(_) = stream.read_exact(&mut len_buf).await {
        return Err(TBGError::DataStreamEarlyEOF)
    }
    let length = u32::from_le_bytes(len_buf) as usize;

    if length > MAX_MESSAGE_LENGTH {
        return Err(TBGError::MessageOverflow(length))
    }
    else if length == 0 {
        return Err(TBGError::EmptyMessage)
    }

    let mut buffer = vec![0; length];

    if let Err(_) = stream.read_exact(&mut buffer).await {
        return Err(TBGError::DataStreamEarlyEOF)
    }

    match serde_json::from_slice::<T>(&buffer[..length]) {
        Ok(message) => Ok(message),
        Err(_) => Err(TBGError::FailDeserialize),
    }
}

pub async fn receive_message_with_timeout<T: for<'de> Deserialize<'de>>(
    stream: &mut TcpStream,
    duration: Duration,
) -> Result<T, TBGError> {
    let timeout_result = timeout(duration, deserialize_message(stream)).await;
    match timeout_result {
        Ok(x) => x,
        Err(_) => Err(TBGError::TimeOut),
    }
}
