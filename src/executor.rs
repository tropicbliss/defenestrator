#![allow(clippy::cast_precision_loss)]

use ansi_term::Colour::Yellow;
use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::{num::NonZeroUsize, time::Duration};

pub async fn run(
    names: Vec<String>,
    parallel_requests: NonZeroUsize,
    timeout: u64,
) -> Result<Vec<String>> {
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let bodies: Vec<_> = stream::iter(names)
        .map(|name| {
            let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", name);
            // Client has its own internal Arc impl so each clone is just cloning a reference to it
            let client = client.clone();
            tokio::spawn(async move {
                let mut attempts = 0;
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
                        },
                        429 => {
                            attempts += 1;
                            if attempts > 3 {
                                panic!("IP is getting rate limited after 3 attempts. Consider raising the timeout");
                            }
                            println!("IP currently rate limited, waiting for {} seconds. Attempt(s): {}", timeout, attempts);
                            tokio::time::sleep(Duration::from_secs(timeout)).await;
                        }
                        _ => panic!("HTTP {}", resp.status()),
                    };
                }
            })
        })
        // Limiting concurrency to prevent OS from running out of resources
        .buffer_unordered(usize::from(parallel_requests))
        .collect()
        .await;
    let mut available_names = Vec::new();
    for body in bodies {
        let body = body?;
        if let NameResult::Available(name) = body {
            available_names.push(name);
        }
    }
    Ok(available_names)
}

enum NameResult {
    Available(String),
    Taken,
}
