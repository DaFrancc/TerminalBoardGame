use crate::data::PlayerSerialize;
use crate::error::TBGError;
use crate::message::{serialize_message, StartMessage};
use serde::Serialize;
use std::io;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub async fn join_server_with_ip<T: Serialize>(ip: String) -> Result<(), TBGError> {
    let mut name = String::new();
    loop {
        println!("Enter your name:");

        match io::stdin().read_line(&mut name) {
            Ok(_) => break,
            Err(error) => println!("error: {}\nTry again", error),
        }
    }

    name = name.trim().to_string();
    if name.is_empty() {
        return Err(TBGError::Other(String::from("Name cannot be empty")));
    }
    let mut stream = match TcpStream::connect(ip).await {
        Ok(x) => x,
        Err(e) => return Err(TBGError::FailToConnect(e.to_string())),
    };

    let players: Vec<PlayerSerialize<Option<Vec<u8>>>> = vec![];
    stream
        .write_all(serialize_message(&StartMessage::<T>::Join(name.clone())).as_slice())
        .await?;

    Ok(())
}

