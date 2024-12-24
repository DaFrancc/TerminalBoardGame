use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, MutexGuard, RwLock};
use crate::error::TBGError;

const LOOKUP_SIZE: usize = 128;
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
    players: RwLock<Vec<Arc<Mutex<PlayerInfo<T>>>>>,
    lookup: RwLock<[Option<usize>; LOOKUP_SIZE]>
}

impl<T: Clone> PlayersList<T> {
    pub(crate) fn new() -> Self {
        Self { players: RwLock::new(Vec::new()), lookup: RwLock::new([None; LOOKUP_SIZE]) }
    }
    pub(crate) async fn add_player(&self, name: String, stream: TcpStream, data: Option<T>) -> Result<usize, (TcpStream, TBGError)> {
        if !self.contains_player(name.clone()).await {
            let mut players = self.players.write().await;
            let id = LOOKUP_INDEX.fetch_add(1, Ordering::SeqCst);
            players
                .push(Arc::new(Mutex::new(PlayerInfo {name, id, start_game: false, stream, data})));
            self.lookup.write().await[id] = Some(players.len() - 1);
            return Ok(id)
        }
        Err((stream, TBGError::InvalidName))
    }
    pub(crate) async fn remove_player(&self, id: usize) {
        let mut players = self.players.write().await;
        let mut lookup = self.lookup.write().await;
        match lookup[id] {
            Some(idx) => {
                players.remove(idx);
                lookup[id] = None;
                for i in idx..players.len() {
                    lookup[players[i].lock().await.id] = Some(i);
                }
            },
            None => return,
        }
    }
    pub(crate) async fn contains_player(&self, other_name: String) -> bool {
        for player in self.players.read().await.iter() {
            if player.lock().await.name == other_name {
                return true;
            }
        }
        false
    }

    pub(crate) async fn read_players(&self) -> tokio::sync::RwLockReadGuard<'_, Vec<Arc<Mutex<PlayerInfo<T>>>>> {
        self.players.read().await
    }

    pub(crate) async fn get_player(&self, id: usize) -> Option<MutexGuard<PlayerInfo<T>>> {
        let players = self.players.read().await;
        let lookup = self.lookup.read().await;
        match lookup[id] {
            Some(idx) => Some(players[idx].clone().lock().await),
            None => None,
        }
    }
}