use crate::error::PlayerListError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

static LOOKUP_INDEX: AtomicUsize = AtomicUsize::new(0);

// In the future, use fine-grained locking to prevent deadlock
// and to avoid using options
pub(crate) struct PlayerInfo<T: Clone> {
    pub(crate) name: String,
    pub(crate) id: usize,
    pub(crate) start_game: bool,
    pub(crate) stream: TcpStream,
    pub(crate) data: Option<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct PlayerSerialize<T: Clone> {
    pub(crate) name: String,
    pub(crate) data: Option<T>,
}

pub struct PlayerView<'a, T: Clone> {
    _guard: RwLockReadGuard<'a, HashMap<usize, PlayerInfo<T>>>,
    player: &'a PlayerInfo<T>,
}

impl<'a, T: Clone> PlayerView<'a, T> {
    pub fn get(&self) -> &PlayerInfo<T> {
        self.player
    }
}

/// Struct that simplifies accessing the player list. Should not be accessed
/// directly, only through the provided functions.
pub(crate) struct PlayersList<T: Clone> {
    players: RwLock<HashMap<usize, PlayerInfo<T>>>,
}

impl<T: Clone> PlayersList<T> {
    pub(crate) fn new() -> Self {
        Self {
            players: RwLock::new(HashMap::new()),
        }
    }

    /// Adds player to the PlayersList. Returns id of player on success,
    /// returns given stream and an error on failure.
    pub(crate) async fn add_player(
        &self,
        name: String,
        stream: TcpStream,
        data: Option<T>,
    ) -> Result<usize, (TcpStream, PlayerListError)> {
        if !self.contains_player(name.clone()).await {
            let mut players = self.players.write().await;
            let id = LOOKUP_INDEX.fetch_add(1, Ordering::SeqCst);
            players.insert(
                id,
                PlayerInfo {
                    name,
                    id,
                    start_game: false,
                    stream,
                    data,
                },
            );
            return Ok(id);
        }
        Err((stream, PlayerListError::DuplicateEntry))
    }

    /// Removes player. Returns an option
    pub(crate) async fn remove_player(&self, id: usize) -> Result<PlayerInfo<T>, PlayerListError> {
        match self.players.write().await.remove(&id) {
            Some(player) => Ok(player),
            None => Err(PlayerListError::InvalidID),
        }
    }

    /// True if name exists, false otherwise
    pub(crate) async fn contains_player(&self, other_name: String) -> bool {
        let players = self.read_players().await;
        players.values().any(|player| player.name == other_name)
    }

    /// Returns RwLock of the players list
    pub(crate) async fn read_players(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, HashMap<usize, PlayerInfo<T>>> {
        self.players.read().await
    }
}
