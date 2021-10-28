use std::time::Instant;

use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::io::{stdout, Write};

pub async fn run(names: Vec<String>, parallel_requests: usize) -> Result<Vec<String>> {
    let client = Client::builder().build()?;
    let before = Instant::now();
    let name_list_len = names.len();
    let bodies: Vec<_> = stream::iter(names)
        .map(|name| {
            let url = format!("https://api.ashcon.app/mojang/v2/user/{}", name);
            let client = client.clone();
            tokio::spawn(async move {
                let resp = client.get(&url).send().await.expect("Got a reqwest::Error");
                match resp.status().as_u16() {
                    200 => {
                        println!("{} was taken", name);
                        NameResult::Taken
                    }
                    404 => {
                        println!("{} is available", name);
                        NameResult::Available(name)
                    }
                    _ => panic!("HTTP {}", resp.status()),
                }
            })
        })
        // Limiting concurrency to prevent OS from running out of resources
        .buffer_unordered(parallel_requests)
        .collect()
        .await;
    let elapsed = before.elapsed();
    writeln!(stdout())?;
    writeln!(
        stdout(),
        "{:.11} rqs/sec (ESTIMATE) | Took {} seconds | {} requests",
        (name_list_len as f64 / elapsed.as_secs_f64()),
        elapsed.as_secs_f32(),
        name_list_len
    )?;
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
