use std::time::Instant;

use anyhow::Result;
use console::style;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::io::{stdout, Write};

pub async fn run(names: Vec<String>, parallel_requests: usize) -> Result<Vec<String>> {
    let client = Client::builder().build()?;
    let name_list_len = names.len();
    let before = Instant::now();
    let bodies: Vec<_> = stream::iter(names)
        .map(|name| {
            let url = format!("https://api.ashcon.app/mojang/v2/user/{}", name);
            let client = &client;
            async move {
                let resp = client.get(&url).send().await.expect("Got a reqwest::Error");
                match resp.status().as_u16() {
                    200 => {
                        println!("{} was taken", style(name).yellow());
                        NameResult::Taken
                    }
                    404 => {
                        println!("{} is available", style(&name).yellow());
                        NameResult::Available(name)
                    }
                    _ => panic!("HTTP {}", resp.status()),
                }
            }
        })
        // Limiting concurrency to prevent OS from running out of resources
        .buffer_unordered(parallel_requests)
        .collect()
        .await;
    let elapsed = before.elapsed();
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
    let mut available_names = Vec::new();
    for body in bodies {
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
