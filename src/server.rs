use std::sync::Arc;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::time::timeout;
use crate::error::TBGError;
use crate::data::{PlayerInfo, PlayerSerialize};
use crate::message::{deserialize_message, serialize_message, ClientMessage, InternalMessage, ServerStartMessage};

async fn serialize_players<T: Clone>(players_vec: Arc<RwLock<Vec<Arc<PlayerInfo<T>>>>>, include_data: bool) -> Vec<PlayerSerialize<T>> {
    let mut players = players_vec.write().await;
    let mut v = Vec::new();
    for player in players.iter() {
        v.push(PlayerSerialize {
            name: player.name.clone(),
            data: if include_data { Some(player.data.clone()) } else { None },
        });
    }

    v
}

async fn client_join<T: Clone + Serialize>(mut stream: TcpStream,
                     players_vec: Arc<RwLock<Vec<Arc<PlayerInfo<T>>>>>,
                     num_players: Arc<AtomicU8>, initial_data: T)
-> Result<(), TBGError>
{
    let message = deserialize_message(&mut stream).await?.unwrap();

    // Deserialize the received JSON into a Message
    match message {
        ServerStartMessage::Join(name) => {
            // If name already exists, reject client and continue to next request
            if name.is_empty() {
                return Err(TBGError::InvalidName)
            }
            let mut players = players_vec.write().await;
            if players.iter().filter(|p| p.name == name).count() != 0 {
                drop(players);
                eprintln!("Player with name {name} already joined");

                stream.write_all(serialize_message(&ClientMessage::RejectClient(name)).as_slice()).await?;
                return Err(TBGError::InvalidName);
            }
            drop(players);
            stream.write_all(serialize_message(&ClientMessage::SendAll(serialize_players(players_vec.clone(), false).await)).as_slice()).await?;
            println!("{name} joined");
            // Insert client into vector
            players = players_vec.write().await;
            players.push(Arc::new(PlayerInfo {
                name,
                data: initial_data,
                start_game: false,
                stream,
            }));

            num_players.fetch_add(1, Ordering::Relaxed);
            Ok(())
        },
        _ => Ok(())
    }
}
pub async fn client_thread<T, U, V>(stream: TcpStream,
                                    player_vec: Arc<RwLock<Vec<Arc<PlayerInfo<T>>>>>,
                                    num_players: Arc<AtomicU8>,
                                    num_votes: Arc<AtomicU8>,
                                    mut broadcast_rx: Receiver<ClientMessage<T, U>>,
                                    mpsc_tx: Sender<InternalMessage<V>>, initial_data: T) -> Result<(), TBGError>
{
    match client_join(stream, player_vec.clone(), num_players.clone(), ).await {
        Ok(_) => {},
        Err(e) => return Err(e)
    }

    Ok(())
}

pub async fn server_thread<T, U, V>(listen_ip: String, broadcast_capacity: usize, mpsc_capacity: usize, min_players: u8, min_votes_proportion: f64) -> Result<(), TBGError> {
    let listener = match TcpListener::bind(listen_ip).await {
        Ok(x) => x,
        Err(e) => {
            return Err(TBGError::FailToInitializeServer(e.to_string()));
        },
    };

    let players_vec: Arc<RwLock<Vec<Arc<PlayerInfo<T>>>>> = Arc::new(RwLock::new(Vec::new()));
    let num_players_atomic = Arc::new(AtomicU8::new(0u8));
    let num_votes_atomic = Arc::new(AtomicU8::new(0u8));

    // Create a broadcast channel for server-to-clients communication
    let (broadcast_tx, _): (broadcast::Sender<ClientMessage<U>>, broadcast::Receiver<ClientMessage<U>>) = broadcast::channel(broadcast_capacity);

    // Create an mpsc channel for clients-to-server communication
    let (server_tx, mut server_rx): (Sender<InternalMessage<V>>, mpsc::Receiver<InternalMessage<V>>) = mpsc::channel(mpsc_capacity);

    'super_loop: loop {
        num_votes_atomic.store(0, Ordering::SeqCst);
        'lobby_loop: loop {
            if let Ok(msg) = server_rx.try_recv() {
                if let Err(_) = match msg {
                    InternalMessage::VoteStart(p_name, p_vote) =>
                        broadcast_tx.send(ClientMessage::VoteStart(p_name, p_vote, num_votes_atomic.load(Ordering::SeqCst), num_votes_atomic.load(Ordering::SeqCst))),
                    InternalMessage::AcceptClient(p_name) =>
                        broadcast_tx.send(ClientMessage::AcceptClient(p_name)),
                    InternalMessage::PlayerLeft(p_name) =>
                        broadcast_tx.send(ClientMessage::PlayerLeft(p_name)),
                    InternalMessage::Kick(p_name) =>
                        broadcast_tx.send(ClientMessage::Kick(p_name)),
                    InternalMessage::TimeOutClient(p_name) =>
                        broadcast_tx.send(ClientMessage::TimeOutClient(p_name)),
                    _ => Ok(0),
                } {
                    eprintln!("Failed to send broadcast");
                }
                if num_players_atomic.load(Ordering::SeqCst) > min_players
                    && num_votes_atomic.load(Ordering::SeqCst) as f64 /
                    num_players_atomic.load(Ordering::SeqCst) as f64
                    >= min_votes_proportion {
                    if let Err(_) = broadcast_tx.send(ClientMessage::StartGame) {
                        eprintln!("Failed to send start broadcast");
                    }
                    break 'lobby_loop;
                }
            }
            let timeout_result = timeout(Duration::from_millis(100), listener.accept()).await;
            let new_client = match timeout_result {
                Ok(x) => x,
                Err(_) => continue,
            };
            if let Ok((socket, addr)) = new_client {
                println!("New connection from: {}", addr);

                let players_vec_clone = players_vec.clone();
                let num_players_clone = num_players_atomic.clone();
                let num_votes_clone = num_votes_atomic.clone();
                let broadcast_rx_clone = broadcast_tx.subscribe();
                let server_tx_clone = server_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = client_thread(
                        socket,
                        players_vec_clone,
                        num_players_clone,
                        num_votes_clone,
                        broadcast_rx_clone,
                        server_tx_clone
                    ).await {
                        eprintln!("Error with client {}: {}", addr, e);
                    }
                });
            }
        }
    }

    Ok(())
}