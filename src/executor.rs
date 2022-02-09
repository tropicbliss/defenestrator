#![allow(clippy::cast_precision_loss)]

use anyhow::Result;
use console::style;
use futures::{stream, StreamExt};
use parking_lot::Mutex;
use reqwest::Client;
use std::{
    io::{stdout, Write},
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
    let name_list_len = names.len();
    let before = Instant::now();
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
                            println!("{} was taken", style(name).yellow());
                            break NameResult::Taken;
                        }
                        204 => {
                            println!("{} is available", style(&name).yellow());
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
    let elapsed = before.elapsed();
    tx.send(MsgState::Exit)?;
    let mut available_names = Vec::new();
    for body in bodies {
        let body = body?;
        if let NameResult::Available(name) = body {
            available_names.push(name);
        }
    }
    writeln!(stdout())?;
    writeln!(
        stdout(),
        "{:.11} {} | {} {} {} | {} requests",
        style(name_list_len as f64 / elapsed.as_secs_f64()).green(),
        style("rqs/sec (ESTIMATE)").cyan(),
        style("Took").cyan(),
        elapsed.as_secs_f32(),
        style("seconds").cyan(),
        name_list_len
    )?;
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
