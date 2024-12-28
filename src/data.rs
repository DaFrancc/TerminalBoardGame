use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};
use crate::error::TBGError;

static LOOKUP_INDEX: AtomicUsize = AtomicUsize::new(0);

// In the future, use fine-grained locking to prevent deadlock
// and to avoid using options
pub(crate) struct PlayerInfo<T: Clone> {
    pub(crate) name: String,
    pub(crate) id: usize,
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
    players: RwLock<IndexMap<usize, Arc<Mutex<PlayerInfo<T>>>>>,
}

impl<T: Clone> PlayersList<T> {
    pub(crate) fn new() -> Self {
        Self { players: RwLock::new(IndexMap::new()) }
    }
    pub(crate) async fn add_player(&self, name: String, stream: TcpStream, data: Option<T>) -> Result<usize, (TcpStream, TBGError)> {
        if !self.contains_player(name.clone()).await {
            let mut players = self.players.write().await;
            let id = LOOKUP_INDEX.fetch_add(1, Ordering::SeqCst);
            players.insert(id, Arc::new(Mutex::new(PlayerInfo {name, id, start_game: false, stream, data})));
            return Ok(id)
        }
        Err((stream, TBGError::InvalidName))
    }
    pub(crate) async fn remove_player(&self, id: usize) -> Option<Arc<Mutex<PlayerInfo<T>>>> {
        self.players.write().await.shift_remove(&id)
    }
    pub(crate) async fn contains_player(&self, other_name: String) -> bool {
        // self.read_players().await.values().any(async |p| p.lock().await.name == other_name)
        let players = self.read_players().await;
        for player in players.values() {
            if player.lock().await.name == other_name {
                return true
            }
        }
        false
    }

    pub(crate) async fn read_players(&self) -> tokio::sync::RwLockReadGuard<'_, IndexMap<usize, Arc<Mutex<PlayerInfo<T>>>>> {
        self.players.read().await
    }

    pub(crate) async fn get_player<'a>(
        &'a self,
        id: usize,
    ) -> Option<(RwLockReadGuard<'a, IndexMap<usize, Arc<Mutex<PlayerInfo<T>>>>>, Arc<Mutex<PlayerInfo<T>>>)> {
        let players = self.players.read().await; // Acquire the read lock
        match players.clone().get(&id).clone() {
            Some(p) => Some((players, p.clone())),
            None => None
        } // Clone the Arc for the player
    }
}