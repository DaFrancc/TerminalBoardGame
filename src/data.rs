use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

// In the future, use fine-grained locking to prevent deadlock
// and to avoid using options
pub(crate) struct PlayerInfo<T: Clone> {
    pub(crate) name: String,
    pub(crate) start_game: bool,
    pub(crate) stream: TcpStream,
    pub(crate) data: T
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct PlayerSerialize<T: Clone> {
    pub(crate) name: String,
    pub(crate) data: Option<T>,
}