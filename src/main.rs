use anyhow::{Result, anyhow};
use clap::Parser;
use reqwest::Client;
use robotstxt::DefaultMatcher;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use tokio::fs;
use tokio::time::{Duration, sleep};
use tracing::{info, warn, error, debug};
use tracing_subscriber;
use url::Url;
use regex::Regex;
use std::sync::LazyLock;

static CSS_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"url\(\s*['"]?([^'")]*)['"]?\s*\)|@import\s+['"]([^'"]+)['"]"#).unwrap()
});

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    index: String,

    #[arg(long)]
    output: PathBuf,

    #[arg(long, default_value_t = 3)]
    depth: usize,
}

#[derive(Clone, Debug)]
struct Job {
    url: Url,
    depth: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("Starting sequential crawler with index: {}, output: {}, depth: {}", 
        args.index, args.output.display(), args.depth);

    fs::create_dir_all(&args.output).await?;

    let root = Url::parse(&args.index)?;

    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .user_agent("rust-crawler/0.1")
        .build()?;

    info!("Fetching robots.txt...");
    let robots_txt = fetch_robots_txt(&client, &root).await;
    if !robots_txt.is_empty() {
        info!("Loaded robots.txt ({} bytes)", robots_txt.len());
    } else {
        warn!("No robots.txt found or empty");
    }

    let mut visited = HashSet::<String>::new();
    let mut queue = VecDeque::<Job>::new();

    // seed
    queue.push_back(Job {
        url: root.clone(),
        depth: 0,
    });

    while let Some(job) = queue.pop_front() {
        let url_str = job.url.to_string();
        
        if job.depth > args.depth + 1 {
            continue;
        }

        let norm = normalize_url(&job.url);
        if !visited.insert(norm) {
            continue;
        }

        if !allowed_by_robots(&robots_txt, &job.url) {
            warn!("Disallowed by robots.txt: {}", url_str);
            continue;
        }

        info!("Crawling [depth {}]: {}", job.depth, url_str);
        // Polite delay
        sleep(Duration::from_millis(100)).await;

        let should_extract = job.depth <= args.depth;
        match process_url(&client, &root, &args.output, job.clone(), should_extract).await {
            Ok(links) => {
                if should_extract {
                    for next_url in links {
                        queue.push_back(Job {
                            url: next_url,
                            depth: job.depth + 1,
                        });
                    }
                }
            }
            Err(e) => {
                error!("Error processing {}: {}", url_str, e);
            }
        }
    }

    info!("Crawler finished.");
    Ok(())
}

async fn fetch_robots_txt(client: &Client, root: &Url) -> String {
    let mut robots_url = root.clone();
    robots_url.set_path("/robots.txt");
    robots_url.set_query(None);
    robots_url.set_fragment(None);

    match client.get(robots_url).send().await {
        Ok(resp) => resp.text().await.unwrap_or_default(),
        Err(e) => {
            debug!("Failed to fetch robots.txt: {}", e);
            String::new()
        }
    }
}

fn allowed_by_robots(robots_content: &str, url: &Url) -> bool {
    if robots_content.is_empty() {
        return true;
    }
    let mut matcher = DefaultMatcher::default();
    matcher.allowed_by_robots(robots_content, vec!["rust-crawler"], url.as_str())
}

fn normalize_url(url: &Url) -> String {
    let mut url = url.clone();
    url.set_fragment(None);
    url.set_query(None);
    let mut s = url.to_string();
    if s.ends_with('/') {
        s.pop();
    }
    s
}

fn is_subpath(root_path: &str, target_path: &str) -> bool {
    if target_path == root_path {
        return true;
    }
    if !target_path.starts_with(root_path) {
        return false;
    }
    // Ensure it's a true subpath, not just a prefix (e.g., /docs/stable matches /docs/stable/foo but not /docs/stable-other)
    root_path.ends_with('/') || target_path.chars().nth(root_path.len()) == Some('/')
}

fn is_asset(url: &Url, is_link: bool) -> bool {
    if !is_link {
        return true;
    }
    let path = url.path().to_lowercase();
    path.ends_with(".css") || 
    path.ends_with(".js") ||
    path.ends_with(".png") ||
    path.ends_with(".jpg") ||
    path.ends_with(".jpeg") ||
    path.ends_with(".gif") || 
    path.ends_with(".svg") || 
    path.ends_with(".webp") ||
    path.ends_with(".ico") ||

    path.ends_with(".woff") ||
    path.ends_with(".woff2") ||
    path.ends_with(".ttf") ||
    path.ends_with(".otf") || 
    path.ends_with(".pdf") ||
    path.ends_with(".xml") ||
    path.ends_with(".txt")
}


async fn process_url(
    client: &Client,
    root: &Url,
    output: &PathBuf,
    job: Job,
    should_extract: bool,
) -> Result<Vec<Url>> {
    let resp = client.get(job.url.clone()).send().await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow!("Status {}", status));
    }

    let content_type = resp.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let is_html = content_type.contains("text/html");
    let is_css = content_type.contains("text/css") || job.url.path().ends_with(".css");

    let mut bytes = resp.bytes().await?.to_vec();
    
    // Save file
    let path_segments: Vec<_> = job.url.path_segments().map(|s| s.collect::<Vec<_>>()).unwrap_or_else(|| vec!["index.html"]);
    let mut file_path = output.clone();
    for seg in path_segments {
        if !seg.is_empty() {
            file_path.push(seg);
        }
    }
    if file_path.is_dir() || job.url.path().ends_with('/') {
        file_path.push("index.html");
    }
    
    if is_html && file_path.extension().is_none() {
        file_path.set_extension("html");
    }

    let mut extracted = Vec::new();
    if is_html {
        let html_content = String::from_utf8_lossy(&bytes).to_string();
        let document = Html::parse_document(&html_content);
        
        // Handle <base href="...">
        let mut base_url = job.url.clone();
        let base_selector = Selector::parse("base[href]").unwrap();
        if let Some(base_elem) = document.select(&base_selector).next() {
            if let Some(href) = base_elem.value().attr("href") {
                if let Ok(new_base) = job.url.join(href) {
                    base_url = new_base;
                }
            }
        }

        if should_extract {
            // Selectors for various resources
            let selectors = [
                ("a[href]", "href", true),
                ("link[rel=\"stylesheet\"][href]", "href", false),
                ("script[src]", "src", false),
                ("img[src]", "src", false),
                ("img[srcset]", "srcset", false),
                ("source[srcset]", "srcset", false),
                ("link[rel*=\"icon\"]", "href", false),
                ("iframe[src]", "src", false),
                ("embed[src]", "src", false),
            ];

            for (sel_str, attr, is_link_tag) in selectors {
                let selector = Selector::parse(sel_str).unwrap();
                for element in document.select(&selector) {
                    if let Some(val) = element.value().attr(attr) {
                        let urls_to_process = if attr == "srcset" {
                            val.split(',')
                                .filter_map(|part| part.trim().split_whitespace().next())
                                .collect::<Vec<_>>()
                        } else {
                            vec![val]
                        };

                        for url_val in urls_to_process {
                            if let Ok(mut next_url) = base_url.join(url_val) {
                                next_url.set_fragment(None);
                                
                                let same_domain = next_url.domain() == root.domain();
                                let subpath = is_subpath(root.path(), next_url.path());
                                let asset = is_asset(&next_url, is_link_tag);

                                if same_domain && (subpath || asset) {
                                    extracted.push(next_url);
                                }
                            }
                        }
                    }
                }
            }

            // Extract from <style> tags
            let style_selector = Selector::parse("style").unwrap();
            for element in document.select(&style_selector) {
                let style_content = element.inner_html();
                for cap in CSS_URL_RE.captures_iter(&style_content) {
                    let val = cap.get(1).or_else(|| cap.get(2)).map(|m| m.as_str());
                    if let Some(val) = val {
                        if let Ok(mut next_url) = base_url.join(val) {
                            next_url.set_fragment(None);
                            if next_url.domain() == root.domain() {
                                extracted.push(next_url);
                            }
                        }
                    }
                }
            }
        }

        // Hardcode external links
        let mut new_html = html_content;
        let a_selector = Selector::parse("a[href]").unwrap();
        let mut replacements = Vec::new();
        for element in document.select(&a_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(next_url) = base_url.join(href) {
                    let same_domain = next_url.domain() == root.domain();
                    let subpath = is_subpath(root.path(), next_url.path());
                    let asset = is_asset(&next_url, true);

                    if !same_domain || (!subpath && !asset) {
                        replacements.push((href.to_string(), next_url.to_string()));
                    }
                }
            }
        }
        
        for (old_href, absolute_url) in replacements {
            let from = format!("href=\"{}\"", old_href);
            let to = format!("href=\"{}\"", absolute_url);
            new_html = new_html.replace(&from, &to);
        }
        bytes = new_html.into_bytes();
    } else if is_css && should_extract {
        let css_content = String::from_utf8_lossy(&bytes);
        for cap in CSS_URL_RE.captures_iter(&css_content) {
            let val = cap.get(1).or_else(|| cap.get(2)).map(|m| m.as_str());
            if let Some(val) = val {
                if let Ok(mut next_url) = job.url.join(val) {
                    next_url.set_fragment(None);
                    if next_url.domain() == root.domain() {
                        extracted.push(next_url);
                    }
                }
            }
        }
    }

    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&file_path, &bytes).await?;
    debug!("Saved: {}", file_path.display());

    if !extracted.is_empty() {
        debug!("Found {} resources on {}", extracted.len(), job.url);
    }

    Ok(extracted)
}
