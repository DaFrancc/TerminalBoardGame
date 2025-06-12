use crate::data::PlayerSerialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum GameEvent<T: Clone> {
    PlayerJoined(String),
    PlayerLeft(String),
    VoteStartGame(bool),
    StartGame,
    AcceptClient(String),
    RejectClient(String),
    TimeoutClient(String),
    Kick(String),
    SendAll(Vec<PlayerSerialize<T>>),
    PlayerWon(String),
    PlayerTurn(String),
    Wager(u8, u8),
    CallLiar(String, u8),
    CallExact(String, u8),
    DiceReady,
    ExactCallCorrect(String),
    PlayerBustedOut(String),
    PlayerLostDie(String),
    ShuffleResult(Vec<u8>),
    Custom(T),
    // You can use T for extensibility
}

pub struct EventBus<T: Clone> {
    sender: broadcast::Sender<GameEvent<T>>,
}

impl<T: Clone + Send + Sync + 'static> EventBus<T> {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(128);
        Self { sender }
    }

    pub fn emit(&self, event: GameEvent<T>) {
        let _ = self.sender.send(event);
    }

    pub fn register_handler<F>(&self, mut handler: F)
    where
        F: FnMut(GameEvent<T>) + Send + 'static,
    {
        let mut rx = self.sender.subscribe();

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                handler(event.clone());
            }
        });
    }
}
