#![allow(clippy::cast_precision_loss)]

use ansi_term::Colour::Yellow;
use anyhow::Result;
use futures::{stream, StreamExt};
use parking_lot::Mutex;
use reqwest::Client;
use std::{
    num::NonZeroUsize,
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};
use tokio::time::sleep;

pub async fn run(
    names: Vec<String>,
    parallel_requests: NonZeroUsize,
    delay: u64,
) -> Result<Vec<String>> {
    let client = Client::builder().build()?;
    let (tx, rx) = mpsc::channel();
    let start_time = Instant::now();
    let start_time = Arc::new(Mutex::new(start_time));
    tokio::spawn(async move {
        let mut state = MsgState::Unlocked;
        while let Ok(item) = rx.recv() {
            if item == MsgState::Locked && state == MsgState::Unlocked {
                state = MsgState::Locked;
                println!(
                    "Request rate limited, waiting {} seconds before reattempting...",
                    delay
                );
            }
            if item == MsgState::Unlocked && state == MsgState::Locked {
                state = MsgState::Unlocked;
            }
            if item == MsgState::Exit {
                break;
            }
        }
    });
    let bodies: Vec<_> = stream::iter(names)
        .map(|name| {
            let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", name);
            // Client has its own internal Arc impl so each clone is just cloning a reference to it
            let client = client.clone();
            let tx = tx.clone();
            let start_time = Arc::clone(&start_time);
            tokio::spawn(async move {
                loop {
                    let resp = client
                        .get(&url)
                        .send()
                        .await
                        .expect("Error while sending request");
                    match resp.status().as_u16() {
                        200 => {
                            println!("{} was taken", Yellow.paint(name));
                            break NameResult::Taken;
                        }
                        204 => {
                            println!("{} is available", Yellow.paint(&name));
                            break NameResult::Available(name);
                        }
                        429 => {
                            tx.send(MsgState::Locked).unwrap();
                            let end_time = Instant::now();
                            let mut start_time_now = Instant::now();
                            {
                                let mut start_time = start_time.lock();
                                std::mem::swap(&mut start_time_now, &mut start_time);
                            }
                            let time_to_wait = Duration::from_secs(delay)
                                .checked_sub(end_time - start_time_now)
                                .unwrap_or(Duration::ZERO);
                            sleep(time_to_wait).await;
                            tx.send(MsgState::Unlocked).unwrap();
                        }
                        _ => panic!("HTTP {}", resp.status()),
                    }
                }
            })
        })
        // Limiting concurrency to prevent OS from running out of resources
        .buffer_unordered(usize::from(parallel_requests))
        .collect()
        .await;
    tx.send(MsgState::Exit)?;
    let mut available_names = Vec::new();
    for body in bodies {
        let body = body?;
        if let NameResult::Available(name) = body {
            available_names.push(name);
        }
    }
    Ok(available_names)
}

#[derive(PartialEq)]
enum MsgState {
    Locked,
    Unlocked,
    Exit,
}

enum NameResult {
    Available(String),
    Taken,
}
