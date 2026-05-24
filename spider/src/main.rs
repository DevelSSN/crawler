mod models;
mod crawler;

use anyhow::Result;
use clap::Parser;
use reqwest::Client;
use std::collections::{HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
use tracing::{info, warn, error};
use url::Url;
use ipnet::IpNet;

use reqwest::dns::{Resolve, Resolving, Name};

struct CustomResolver {
    forbidden: Vec<IpNet>,
}

impl Resolve for CustomResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let forbidden = self.forbidden.clone();
        let name_str = name.as_str().to_string();
        Box::pin(async move {
            // lookup_host typically takes "host:port", but reqwest Name might not include port or might be handled differently.
            // Actually Name::as_str() is just the hostname. We might need a default port or assume reqwest handles it.
            // reqwest documentation shows Resolve::resolve takes Name.
            let addrs = tokio::net::lookup_host(format!("{}:0", name_str)).await?;
            let filtered: Vec<SocketAddr> = addrs.filter(|addr| {
                let ip = addr.ip();
                !forbidden.iter().any(|net| net.contains(&ip))
            }).collect();
            
            if filtered.is_empty() {
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Access to private/internal IP denied")) as Box<dyn std::error::Error + Send + Sync>)
            } else {
                Ok(Box::new(filtered.into_iter()) as Box<dyn Iterator<Item = SocketAddr> + Send>)
            }
        })
    }
}

use crate::models::{Args, Job};
use crate::crawler::{allowed_by_robots, fetch_robots_txt, normalize_url, process_url};

struct WorkerResult {
    links: Vec<Url>,
    parent_depth: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Arc::new(Args::parse());

    info!("Starting multithreaded crawler with index: {}, output: {}, depth: {}, workers: {}", 
        args.index, args.output.display(), args.depth, args.workers);

    fs::create_dir_all(&args.output).await?;

    let root = Url::parse(&args.index)?;
    
    let forbidden_ranges: Vec<IpNet> = [
        "127.0.0.0/8", "10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16",
        "169.254.0.0/16", "::1/128", "fc00::/7", "fe80::/10"
    ].iter().map(|s| s.parse().unwrap()).collect();

    let client = Arc::new(Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .user_agent("rust-crawler/0.1")
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(5))
        .dns_resolver(Arc::new(CustomResolver { forbidden: forbidden_ranges }))
        .build()?);

    info!("Fetching robots.txt...");
    let robots_txt = Arc::new(fetch_robots_txt(&client, &root).await);
    if !robots_txt.is_empty() {
        info!("Loaded robots.txt ({} bytes)", robots_txt.len());
    } else {
        warn!("No robots.txt found or empty");
    }

    let mut visited = HashSet::<String>::new();
    let mut queue = VecDeque::<Job>::new();
    let workers = args.workers.max(1);
    let (tx, mut rx) = mpsc::channel::<WorkerResult>(workers * 2);

    // Seed
    queue.push_back(Job {
        url: root.clone(),
        depth: 0,
    });

    let mut active_workers = 0;

    loop {
        // 1. Dispatch work if we have capacity and jobs
        while active_workers < workers && !queue.is_empty() {
            if let Some(job) = queue.pop_front() {
                let norm = normalize_url(&job.url);
                if !visited.insert(norm) {
                    continue;
                }

                if !allowed_by_robots(&robots_txt, &job.url) {
                    warn!("Disallowed by robots.txt: {}", job.url);
                    continue;
                }

                active_workers += 1;
                let tx_clone = tx.clone();
                let client_clone = Arc::clone(&client);
                let root_clone = root.clone();
                let args_clone = Arc::clone(&args);
                let job_clone = job.clone();

                tokio::spawn(async move {
                    let url_str = job_clone.url.to_string();
                    info!("Crawling [depth {}]: {}", job_clone.depth, url_str);
                    
                    // Polite delay
                    sleep(Duration::from_millis(100)).await;

                    let should_extract = job_clone.depth < args_clone.depth;
                    let result = process_url(
                        &client_clone, 
                        &root_clone, 
                        &args_clone.output, 
                        job_clone.clone(), 
                        should_extract,
                        args_clone.hardcode_external,
                    ).await;

                    let links = match result {
                        Ok(l) => l,
                        Err(e) => {
                            error!("Error processing {}: {}", url_str, e);
                            Vec::new()
                        }
                    };

                    let _ = tx_clone.send(WorkerResult {
                        links,
                        parent_depth: job_clone.depth,
                    }).await;
                });
            }
        }

        // 2. Break if nothing is happening
        if active_workers == 0 && queue.is_empty() {
            break;
        }

        // 3. Collect results
        if active_workers > 0 {
            if let Some(result) = rx.recv().await {
                active_workers -= 1;
                
                for next_url in result.links {
                    queue.push_back(Job {
                        url: next_url,
                        depth: result.parent_depth + 1,
                    });
                }
            }
        }
    }

    info!("Crawler finished.");
    Ok(())
}
