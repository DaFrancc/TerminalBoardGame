use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use crate::error::TBGError;

// In the future, use fine-grained locking to prevent deadlock
// and to avoid using options
pub(crate) struct PlayerInfo<T: Clone> {
    pub(crate) name: String,
    pub(crate) start_game: bool,
    pub(crate) stream: TcpStream,
    pub(crate) data: Option<T>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct PlayerSerialize<T: Clone> {
    pub(crate) name: String,
    pub(crate) data: Option<T>,
}

/// Struct that simplifies accessing the player list. Should not be accessed
/// directly, only through the provided functions.
pub(crate) struct PlayersList<T: Clone> {
    players: RwLock<Vec<Arc<PlayerInfo<T>>>>
}

impl<T: Clone> PlayersList<T> {
    pub(crate) fn new() -> Self {
        Self { players: RwLock::new(Vec::new()) }
    }
    pub(crate) async fn add_player(&self, name: String, stream: TcpStream, data: Option<T>) -> Result<(), TBGError> {
        if !self.contains_player(name.clone()).await {
            self
                .players
                .write()
                .await
                .push(Arc::new(PlayerInfo {name, start_game: false, stream, data}));
            return Ok(())
        }
        Err(TBGError::InvalidName)
    }
    pub(crate) async fn remove_player(&self, name: String) {
        self.players.write().await.retain(|p| p.name != name);
    }
    pub(crate) async fn contains_player(&self, other_name: String) -> bool {
        self
            .players
            .read()
            .await
            .iter()
            .any(|elem| elem.name == other_name)
    }

    pub(crate) async fn get_players(&self) -> tokio::sync::RwLockReadGuard<'_, Vec<Arc<PlayerInfo<T>>>> {
        self.players.read().await
    }
}