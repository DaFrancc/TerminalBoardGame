use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use crossterm::style::{style, Attribute, Color, Stylize};
use tokio::time::timeout;
use crate::error::TBGError;
use crate::data::{PlayerInfo, PlayerSerialize, PlayersList};
use crate::message::{receive_message_with_timeout, serialize_message, ClientMessage, ClientStartMessage, InternalMessage, ServerMessage, ServerStartMessage};

async fn serialize_players<T: Clone>(players_list: Arc<PlayersList<T>>, include_data: bool) -> Vec<PlayerSerialize<Option<T>>> {
    let players = players_list.read_players().await;
    let mut v = Vec::new();
    for player in players.iter() {
        v.push(PlayerSerialize {
            name: player.lock().await.name.clone(),
            data: if include_data { Some(player.lock().await.data.clone()) } else { None },
        });
    }

    v
}

async fn client_join<T: Clone + Serialize + for<'a> Deserialize<'a>, U: Serialize>(
    mut stream: TcpStream,
    players_list: Arc<PlayersList<T>>,
    num_players: Arc<AtomicU8>)
    -> Result<usize, TBGError>
{
    let message: ServerStartMessage<T> = match receive_message_with_timeout(&mut stream, Duration::from_secs(30)).await {
        Some(x) => x,
        None => return Err(TBGError::TimeoutError),
    };

    // Deserialize the received JSON into a Message
    match message {
        ServerStartMessage::Join(name) => {
            // If name already exists, reject client and continue to next request
            if name.is_empty() {
                return Err(TBGError::InvalidName)
            }
            let mut player_id = match players_list.add_player(name.clone(), stream, None).await {
                Ok(x) => x,
                Err((mut s, _)) => {
                    eprintln!("Player with name {name} already joined");

                    s.write_all(serialize_message(&ClientMessage::<T, U>::RejectClient(name)).as_slice()).await?;
                    s.flush().await.unwrap_or_else(|e| eprintln!("Failed to flush client to disk: {}", e));
                    return Err(TBGError::InvalidName);
                },
            };
            let players_lock = players_list.read_players().await;
            let player = match players_list.get_player(player_id).await {
                Some(p) => p,
                None => return Err(TBGError::Other(format!("Player with name {} not found. Failed to add to player list", name))),
            };
            player.stream.write_all(serialize_message(&ClientMessage::<Option<T>, U>::SendAll(serialize_players(players_list.clone(), false).await)).as_slice()).await?;
            println!("{name} joined");

            num_players.fetch_add(1, Ordering::Relaxed);
            Ok(player_id)
        },
        _ => Err(TBGError::FailToConnect(stream.peer_addr()?.to_string())),
    }
}
pub async fn client_thread<T: Clone + Serialize + for<'a> Deserialize<'a>, U: Serialize>(
    stream: TcpStream,
    players: Arc<PlayersList<T>>,
    num_players: Arc<AtomicU8>,
    num_votes: Arc<AtomicU8>,
    mut broadcast_rx: Receiver<ClientMessage<T, U>>,
    mpsc_tx: Sender<InternalMessage<U>>) -> Result<(), TBGError>
{
    let player_id = match client_join::<T, U>(stream, players.clone(), num_players.clone()).await {
        Ok(id) => id,
        Err(e) => return Err(e)
    };

    match client_in_lobby::<T, U>(player_id, players.clone(), num_players.clone(), num_votes.clone(), broadcast_rx, mpsc_tx).await {
        Ok(_) => {},
        Err(e) => return Err(e)
    }

    Ok(())
}

async fn client_in_lobby<T: Clone + Serialize + for<'a> Deserialize<'a>, U: Serialize>(
    player_id: usize,
    players_list: Arc<PlayersList<T>>,
    num_players: Arc<AtomicU8>,
    num_votes: Arc<AtomicU8>,
    mut broadcast_rx: Receiver<ClientMessage<T, U>>,
    mpsc_tx: Sender<InternalMessage<U>>) -> Result<(), TBGError>
{
    let name = {
        let players_lock = players_list.read_players().await;
        let player = match players_list.get_player(player_id).await {
            Some(p) => p,
            None => return Err(TBGError::Other(format!("Player with id {} not found. Failed to add to player list", player_id))),
        };
        player.name.clone()
    };

    'lobby_loop: loop {
        if let Ok(msg) = broadcast_rx.try_recv() {
            let serialized: Result<ClientMessage<T, U>, TBGError> = match msg {
                ClientMessage::StartGame => Ok(ClientMessage::StartGame),
                ClientMessage::PlayerLeft(p_name) => Ok(ClientMessage::PlayerLeft(p_name)),
                ClientMessage::AcceptClient(p_name) => Ok(ClientMessage::AcceptClient(p_name)),
                ClientMessage::VoteStart(p_name, p_vote, n_votes, n_players) =>
                    Ok(ClientMessage::VoteStart(p_name, p_vote, n_votes, n_players)),
                ClientMessage::Kick(p_name) => Ok(ClientMessage::Kick(p_name)),
                _ => Err(TBGError::Other(String::from("Unknown message type"))),
            };
            match serialized {
                Ok(s) => {
                    let players_lock = players_list.read_players().await;
                    let player = match players_list.get_player(player_id).await {
                        Some(p) => p,
                        None => return Err(TBGError::Other(format!("Player with name {} not found. Failed to add to player list", name))),
                    };
                    player.stream.write_all(serialize_message(&s).as_slice()).await?;

                    match s {
                        ClientMessage::Kick(p_name) if p_name == name => {
                            return Err(TBGError::Other(String::from("Player kicked")));
                        },
                        ClientMessage::StartGame => {
                            return Ok(())
                        },
                        _ => {},
                    }
                }
                Err(_) => eprintln!("Failed to send message to client"),
            }
        }

        let players = players_list.read_players().await;
        let player = match players_list.get_player(player_id).await {
            Some(p) => p,
            None => return Err(TBGError::Other(format!("Player with id {} not found. Failed to add to player list", player_id))),
        };
        let message = match receive_message_with_timeout::<ServerStartMessage<U>>(
            &mut player.stream, Duration::from_millis(20)).await
        {
            Some(m) => m,
            None => continue,
        };
        // Read message content
        match message {
            ServerStartMessage::VoteStart(vote) => {
                if vote == player.start_game {
                    continue
                } else if vote && !player.start_game {
                    num_votes.fetch_add(1, Ordering::SeqCst);
                } else if !vote && player.start_game {
                    num_votes.fetch_sub(1, Ordering::SeqCst);
                }
                player.start_game = vote;
                let n_votes = num_votes.load(Ordering::SeqCst);
                let n_people = num_players.load(Ordering::SeqCst);

                if let Err(_) = mpsc_tx.send(InternalMessage::VoteStart(player.name.clone(), vote)).await {
                    eprintln!("Failed to send message to server");
                }

                print!("{} voted ", player.name);
                if vote {
                    print!("{}", style("YES").with(Color::Green).attribute(Attribute::Bold).attribute(Attribute::Underlined));
                } else {
                    print!("{}", style("NO").with(Color::Red).attribute(Attribute::Bold).attribute(Attribute::Underlined));
                }
                println!(" to start game ({}/{})", n_votes, n_people);
            }
            ServerStartMessage::Exit => {
                if let Err(_) = mpsc_tx.send(InternalMessage::PlayerLeft(player.name.clone())).await {
                    eprintln!("Failed to send message to server");
                }
                println!("{} left", player.name);
                num_players.fetch_sub(1, Ordering::SeqCst);
                if player.start_game {
                    num_votes.fetch_sub(1, Ordering::SeqCst);
                }
                players_list.remove_player(player_id).await;
                return Ok(())
            }
            _ => continue,
        }
    }
}

/// Master server thread that handles main game logic and loop and handles
/// input and output to its client threads.
/// T - Custom player data (i.e. dice, cards, other game attributes)
/// U - Custom player actions
pub async fn server_thread<T: Clone + Serialize + for<'a> Deserialize<'a> + Send + Sync + 'static, U: Clone + Serialize + Send + 'static>(
    listen_ip: String,
    broadcast_capacity: usize,
    mpsc_capacity: usize,
    min_players: u8,
    min_votes_proportion: f64
) -> Result<(), TBGError> {
    let listener = match TcpListener::bind(listen_ip).await {
        Ok(x) => x,
        Err(e) => {
            return Err(TBGError::FailToInitializeServer(e.to_string()));
        },
    };
    /* The type is long and complicated, but this should allow every thread
     to access its players without locking up the entire program. */
    let mut players_list: Arc<PlayersList<T>> = Arc::new(PlayersList::new());
    let num_players_atomic = Arc::new(AtomicU8::new(0u8));
    let num_votes_atomic = Arc::new(AtomicU8::new(0u8));

    // Create a broadcast channel for server-to-clients communication
    let (broadcast_tx, _): (broadcast::Sender<ClientMessage<T, U>>, broadcast::Receiver<ClientMessage<T, U>>) = broadcast::channel(broadcast_capacity);

    // Create an mpsc channel for clients-to-server communication
    let (server_tx, mut server_rx): (Sender<InternalMessage<U>>, mpsc::Receiver<InternalMessage<U>>) = mpsc::channel(mpsc_capacity);

    'super_loop: loop {
        num_votes_atomic.store(0, Ordering::SeqCst);
        'lobby_loop: loop {
            if receive_client_message(
                min_players,
                min_votes_proportion,
                num_players_atomic.clone(),
                num_votes_atomic.clone(),
                &broadcast_tx,
                &mut server_rx
            ) {
                break 'lobby_loop;
            }

            accept_new_client(
                &listener,
                players_list.clone(),
                num_players_atomic.clone(),
                num_votes_atomic.clone(),
                broadcast_tx.subscribe(),
                server_tx.clone()
            ).await;
        }
    }
}

async fn accept_new_client<T: Clone + Serialize + for<'a> Deserialize<'a> + Send + Sync + 'static,
    U: Clone + Serialize + Send + 'static>(
    listener: &TcpListener,
    players: Arc<PlayersList<T>>,
    num_players_atomic: Arc<AtomicU8>,
    num_votes_atomic: Arc<AtomicU8>,
    broadcast_rx: Receiver<ClientMessage<T, U>>,
    server_tx: Sender<InternalMessage<U>>)
    -> bool {
    let timeout_result = timeout(Duration::from_millis(100), listener.accept()).await;
    let new_client = match timeout_result {
        Ok(x) => x,
        Err(_) => return true,
    };
    if let Ok((socket, addr)) = new_client {
        println!("New connection from: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = client_thread(
                socket,
                players,
                num_players_atomic,
                num_votes_atomic,
                broadcast_rx,
                server_tx
            ).await {
                eprintln!("Error with client {}: {}", addr, e);
            }
        });
    }
    false
}

fn receive_client_message<T: Clone + Serialize + for<'a> Deserialize<'a> + Send + Sync,
    U: Clone + Serialize + Send>(
    min_players: u8,
    min_votes_proportion: f64,
    num_players_atomic: Arc<AtomicU8>,
    num_votes_atomic: Arc<AtomicU8>,
    broadcast_tx: &broadcast::Sender<ClientMessage<T, U>>,
    server_rx: &mut mpsc::Receiver<InternalMessage<U>>) -> bool {
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
            return true;
        }
    }
    false
}