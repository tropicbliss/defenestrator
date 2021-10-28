use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;

pub async fn run(names: Vec<String>, parallel_requests: usize) -> Result<Vec<String>> {
    let client = Client::builder().build()?;
    let bodies: Vec<_> = stream::iter(names)
        .map(|name| {
            let url = format!("https://api.ashcon.app/mojang/v2/user/{}", name);
            let client = &client;
            async move {
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
            }
        })
        // Limiting parallelism to prevent OS from running out of resources
        .buffer_unordered(parallel_requests)
        .collect()
        .await;
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
