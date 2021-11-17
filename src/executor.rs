use ansi_term::Colour::Yellow;
use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::{
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub async fn run(
    names: Vec<String>,
    parallel_requests: NonZeroUsize,
    timeout: u64,
) -> Result<Vec<String>> {
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let show_ratelimit_msg = Arc::new(AtomicBool::new(true));
    let bodies: Vec<_> = stream::iter(names)
        .map(|name| {
            // Client has its own internal Arc impl so each clone is just cloning a reference to it
            let client = client.clone();
            // Makes sure rate limit msg appears once (a bit of overengineering I know but I wanna learn atomics)
            let show_ratelimit_msg = Arc::clone(&show_ratelimit_msg);
            tokio::spawn(async move {
                let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", name);
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
                            // If CAS succeeds, it will return Ok. If it fails, it will return Err.
                            // In ARM, instead of CAS, we have LDREX (get exclusive access to a memory location and read it) and STREX (get exclusive access to a memory location and store the value, but has the possibility of failing if it is unable to get exclusive access). Since in our case, if a thread has exclusive access to the value, the other threads should be fine not attempting to store the value, we can use weak safely (among many other kinds of spurious errors). Plus, printing a rate limit msg is hardly mission critical.
                            if show_ratelimit_msg.compare_exchange_weak(true, false, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                                println!("IP currently rate limited, waiting for {} seconds. Attempt: {}/3", timeout, attempts);
                            }
                            tokio::time::sleep(Duration::from_secs(timeout)).await;
                            show_ratelimit_msg.store(true, Ordering::Release);
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
