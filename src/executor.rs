use crate::utils;
use ansi_term::Colour::Yellow;
use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashSet,
    io::{self, Write},
    num::NonZeroUsize,
    time::Duration,
};
use tokio::{sync::mpsc, time::sleep};

pub async fn run(
    names: Vec<String>,
    parallel_requests: NonZeroUsize,
    delay: u64,
) -> Result<Vec<String>> {
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let (tx, mut rx) = mpsc::channel(100);
    let parallel_requests = usize::from(parallel_requests);
    let aux_channel = tokio::spawn(async move {
        let mut state = 0;
        while let Some(item) = rx.recv().await {
            if item == MsgState::RateLimited {
                state += 1;
                if state == parallel_requests {
                    state = 0;
                    println!(
                        "Requests rate limited, waiting {} seconds before reattempting...",
                        delay
                    );
                }
            }
        }
    });
    let names: Vec<HashSet<String>> = names
        .chunks(10)
        .map(|name| name.iter().map(|n| utils::to_title(n)))
        .map(HashSet::from_iter)
        .collect();
    let bodies: Vec<_> = stream::iter(names)
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
                            let blocked_handle = tokio::task::spawn_blocking(move || {
                                let stdout = io::stdout();
                                let handle = stdout.lock();
                                let mut handle = io::BufWriter::new(handle);
                                let result: HashSet<String> = result
                                    .into_iter()
                                    .map(|unit| utils::to_title(&unit.name))
                                    .collect();
                                for name in &result {
                                    writeln!(handle, "{} was taken", Yellow.paint(name)).unwrap();
                                }
                                let available_names: Vec<String> =
                                    name.difference(&result).cloned().collect();
                                for name in &available_names {
                                    writeln!(handle, "{} is available", Yellow.paint(name))
                                        .unwrap();
                                }
                                handle.flush().unwrap();
                                available_names
                            });
                            return blocked_handle.await.unwrap();
                        }
                        429 => {
                            tx.send(MsgState::RateLimited).await.unwrap();
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
    aux_channel.abort();
    let mut available_names = Vec::new();
    for body in bodies {
        let mut body = body?;
        available_names.append(&mut body);
    }
    Ok(available_names)
}

#[derive(PartialEq, Debug)]
enum MsgState {
    RateLimited,
}

#[derive(Deserialize)]
struct Unit {
    name: String,
}
