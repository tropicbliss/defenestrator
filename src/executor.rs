#![allow(clippy::cast_precision_loss)]

use ansi_term::Colour::Yellow;
use anyhow::Result;
use futures::{stream, StreamExt};
use parking_lot::Mutex;
use reqwest::Client;
use std::{
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};
use tokio::time::sleep;

pub async fn run(names: Vec<String>, parallel_requests: usize, delay: u64) -> Result<Vec<String>> {
    let client = Client::builder().build()?;
    let (tx, rx) = mpsc::channel();
    let start_time = Instant::now();
    let start_time = Arc::new(Mutex::new(start_time));
    tokio::spawn(async move {
        let mut state = 0;
        while let Ok(item) = rx.recv() {
            if let MsgState::Locking(delay) = item {
                state += 1;
                if state == parallel_requests {
                    state = 0;
                    println!(
                        "Requests rate limited, waiting {} seconds before reattempting...",
                        delay
                    );
                }
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
                            let mut start_time_m = Instant::now();
                            {
                                let mut start_time = start_time.lock();
                                std::mem::swap(&mut start_time_m, &mut start_time);
                            }
                            let end_time = Instant::now();
                            let time_to_wait = Duration::from_secs(delay)
                                .checked_sub(end_time - start_time_m)
                                .unwrap_or(Duration::ZERO);
                            tx.send(MsgState::Locking(time_to_wait.as_secs())).unwrap();
                            sleep(time_to_wait).await;
                        }
                        _ => panic!("HTTP {}", resp.status()),
                    }
                }
            })
        })
        // Limiting concurrency to prevent OS from running out of resources
        .buffer_unordered(parallel_requests)
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
    Locking(u64),
    Exit,
}

enum NameResult {
    Available(String),
    Taken,
}
