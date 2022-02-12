#![allow(clippy::cast_precision_loss)]

use ansi_term::Colour::Yellow;
use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{sync::mpsc, time::Duration};
use tokio::time::sleep;

pub async fn run(names: Vec<String>, parallel_requests: usize, delay: u64) -> Result<Vec<String>> {
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let (tx, rx) = mpsc::channel();
    tokio::spawn(async move {
        let mut state = 0;
        while let Ok(item) = rx.recv() {
            if item == MsgState::Locking {
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
    let bodies: Vec<_> = stream::iter(names.chunks(10).map(Into::into).collect::<Vec<Vec<_>>>())
        .map(|name| {
            // Client has its own internal Arc impl so each clone is just cloning a reference to it
            let client = client.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                loop {
                    let json = json!(name);
                    let resp = client
                        .post("https://api.mojang.com/profiles/minecraft")
                        .json(&json)
                        .send()
                        .await
                        .expect("Error while sending request");
                    match resp.status().as_u16() {
                        200 => {
                            let result: Vec<Unit> = resp.json().await.unwrap();
                            let result: Vec<String> =
                                result.into_iter().map(|unit| unit.name).collect();
                            for name in &result {
                                println!("{} was taken", Yellow.paint(name));
                            }
                            break result;
                        }
                        429 => {
                            tx.send(MsgState::Locking).unwrap();
                            sleep(Duration::from_secs(delay)).await;
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
        available_names.extend(body);
    }
    Ok(available_names)
}

#[derive(PartialEq)]
enum MsgState {
    Locking,
    Exit,
}

#[derive(Deserialize)]
struct Unit {
    name: String,
}
