use anyhow::{Result, anyhow};
use reqwest::Client;
use robotstxt::DefaultMatcher;
use scraper::{Html, Selector};
use std::path::PathBuf;
use url::Url;
use regex::Regex;
use std::sync::LazyLock;
use tokio::fs;
use tracing::debug;
use crate::models::Job;

pub static CSS_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"url\(\s*['"]?([^'")]*)['"]?\s*\)|@import\s+['"]([^'"]+)['"]"#).unwrap()
});

pub async fn fetch_robots_txt(client: &Client, root: &Url) -> String {
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

pub fn allowed_by_robots(robots_content: &str, url: &Url) -> bool {
    if robots_content.is_empty() {
        return true;
    }
    let mut matcher = DefaultMatcher::default();
    matcher.allowed_by_robots(robots_content, vec!["rust-crawler"], url.as_str())
}

pub fn normalize_url(url: &Url) -> String {
    let mut url = url.clone();
    url.set_fragment(None);
    url.set_query(None);
    let mut s = url.to_string();
    if s.ends_with('/') {
        s.pop();
    }
    s
}

pub fn is_subpath(root_path: &str, target_path: &str) -> bool {
    if target_path == root_path {
        return true;
    }
    if !target_path.starts_with(root_path) {
        return false;
    }
    root_path.ends_with('/') || target_path.chars().nth(root_path.len()) == Some('/')
}

pub fn is_asset(url: &Url, is_link: bool) -> bool {
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

pub async fn process_url(
    client: &Client,
    root: &Url,
    output: &PathBuf,
    job: Job,
    should_extract: bool,
) -> Result<Vec<Url>> {
    let (bytes, final_url, is_html, is_css) = fetch_url(client, &job.url).await?;

    let file_path = get_storage_path(output, &job.url, is_html);

    let extracted = if should_extract {
        extract_links(&bytes, &final_url, root, is_html, is_css)
    } else {
        Vec::new()
    };

    save_to_disk(&file_path, &bytes).await?;

    Ok(extracted)
}

async fn fetch_url(client: &Client, url: &Url) -> Result<(Vec<u8>, Url, bool, bool)> {
    let resp = client.get(url.clone()).send().await?;
    let final_url = resp.url().clone();
    
    if !resp.status().is_success() {
        return Err(anyhow!("Status {} for {}", resp.status(), url));
    }

    let content_type = resp.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    let is_html = content_type.contains("text/html");
    let is_css = content_type.contains("text/css") || final_url.path().ends_with(".css");
    let bytes = resp.bytes().await?.to_vec();

    Ok((bytes, final_url, is_html, is_css))
}

fn get_storage_path(output: &PathBuf, url: &Url, is_html: bool) -> PathBuf {
    let path_segments: Vec<_> = url.path_segments()
        .map(|s| s.collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["index.html"]);
    
    let mut file_path = output.clone();
    for seg in path_segments {
        if !seg.is_empty() {
            file_path.push(seg);
        }
    }

    if file_path.is_dir() || url.path().ends_with('/') {
        file_path.push("index.html");
    }
    
    if is_html && file_path.extension().is_none() {
        file_path.set_extension("html");
    }
    file_path
}

fn extract_links(bytes: &[u8], base: &Url, root: &Url, is_html: bool, is_css: bool) -> Vec<Url> {
    if is_html {
        extract_from_html(bytes, base, root)
    } else if is_css {
        extract_from_css(bytes, base, root)
    } else {
        Vec::new()
    }
}

fn extract_from_html(bytes: &[u8], final_url: &Url, root: &Url) -> Vec<Url> {
    let html_content = String::from_utf8_lossy(bytes);
    let document = Html::parse_document(&html_content);
    let mut extracted = Vec::new();

    let mut base_url = final_url.clone();
    let base_selector = Selector::parse("base[href]").unwrap();
    if let Some(base_elem) = document.select(&base_selector).next() {
        if let Some(href) = base_elem.value().attr("href") {
            if let Ok(new_base) = final_url.join(href) {
                base_url = new_base;
            }
        }
    }

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
                let parts: Vec<_> = if attr == "srcset" {
                    val.split(',').filter_map(|p| p.trim().split_whitespace().next()).collect()
                } else {
                    vec![val]
                };

                for url_val in parts {
                    if let Ok(mut next_url) = base_url.join(url_val) {
                        next_url.set_fragment(None);
                        if next_url.domain() == root.domain() && 
                           (is_subpath(root.path(), next_url.path()) || is_asset(&next_url, is_link_tag)) {
                            extracted.push(next_url);
                        }
                    }
                }
            }
        }
    }

    let style_selector = Selector::parse("style").unwrap();
    for element in document.select(&style_selector) {
        extracted.extend(extract_from_css(element.inner_html().as_bytes(), &base_url, root));
    }

    extracted
}

fn extract_from_css(bytes: &[u8], base_url: &Url, root: &Url) -> Vec<Url> {
    let css_content = String::from_utf8_lossy(bytes);
    let mut extracted = Vec::new();
    for cap in CSS_URL_RE.captures_iter(&css_content) {
        if let Some(val) = cap.get(1).or_else(|| cap.get(2)).map(|m| m.as_str()) {
            if let Ok(mut next_url) = base_url.join(val) {
                next_url.set_fragment(None);
                if next_url.domain() == root.domain() {
                    extracted.push(next_url);
                }
            }
        }
    }
    extracted
}

async fn save_to_disk(path: &PathBuf, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, bytes).await?;
    debug!("Saved: {}", path.display());
    Ok(())
}
