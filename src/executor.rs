use anyhow::Result;
use futures::{stream, StreamExt};
use reqwest::Client;
use std::{
    sync::{atomic::AtomicBool, atomic::Ordering, Arc},
    time::Duration,
};

pub async fn run(
    names: Vec<String>,
    parallel_requests: usize,
    timeout: u64,
) -> Result<Vec<String>> {
    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
    let show_ratelimit_msg = Arc::new(AtomicBool::new(true));
    let bodies: Vec<_> = stream::iter(names)
        .map(|name| {
            let url = format!("https://api.mojang.com/users/profiles/minecraft/{}", name);
            // Client has its own internal Arc impl so each clone is just cloning a reference to it
            let client = client.clone();
            // Makes sure rate limit msg appears once (a bit of overengineering I know but I wanna learn atomics)
            let show_ratelimit_msg = Arc::clone(&show_ratelimit_msg);
            tokio::spawn(async move {
                loop {
                    let resp = client.get(&url).send().await.expect("Got a reqwest::Error");
                    match resp.status().as_u16() {
                        200 => {
                            println!("{} was taken", name);
                            break NameResult::Taken;
                        }
                        204 => {
                            println!("{} is available", name);
                            break NameResult::Available(name);
                        }
                        429 => {
                            // If CAS succeeds, it will return Ok. If it fails, it will return Err.
                            // In ARM, instead of CAS, we have LDREX (get exclusive access to a memory location and read it)
                            // and STREX (get exclusive access to a memory location and store the value, but has the possibility
                            // of failing if it is unable to get exclusive access). Since in our case, if a thread has exclusive
                            // access to the value, the other threads should be fine not attempting to store the value, we can
                            // use weak safely (among many other kinds of spurious errors). Plus, printing a rate limit msg is
                            // hardly mission critical.
                            if show_ratelimit_msg
                                .compare_exchange_weak(
                                    true,
                                    false,
                                    Ordering::Acquire,
                                    Ordering::Relaxed,
                                )
                                .is_ok()
                            {
                                println!(
                                    "IP currently rate limited, waiting for {} seconds...",
                                    timeout
                                );
                            }
                            tokio::time::sleep(Duration::from_secs(timeout)).await;
                            // Not sure whether using store or CAS is better
                            show_ratelimit_msg.store(true, Ordering::Release);
                        }
                        _ => panic!("HTTP {}", resp.status()),
                    }
                }
            })
        })
        // Limiting parallelism to prevent OS from running out of resources
        .buffer_unordered(parallel_requests)
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
