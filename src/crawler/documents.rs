use scraper::{Html, Selector};
use std::process::Command;
use std::collections::HashMap;
use url::Url;
use chrono::Utc;

#[derive(Debug, Clone, Default)]
pub struct PageMetadata {
    pub url: String,
    pub title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub robots: Option<String>,
    pub lang: Option<String>,
    pub h1: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
    pub og_url: Option<String>,
    pub content_type: Option<String>,
    pub last_modified: Option<String>,
    pub server: Option<String>,
    pub is_protected: bool,
    pub protection_reason: String,
    pub crawl_timestamp: i64,
}

pub fn fetch_metadata(url: &str) -> Result<PageMetadata, Box<dyn std::error::Error>> {
    // --- 1. Use curl (bypasses Cloudflare) ---
    let output = Command::new("curl")
        .arg("-L")
        .arg("--compressed")
        .arg("-m").arg("30")
        .arg("-H").arg("User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .arg("-H").arg("Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .arg("-H").arg("Accept-Language: en-US,en;q=0.9")
        .arg("-H").arg("Accept-Encoding: gzip, deflate, br")
        .arg("-H").arg("Sec-Fetch-Dest: document")
        .arg("-H").arg("Sec-Fetch-Mode: navigate")
        .arg("-H").arg("Sec-Fetch-Site: none")
        .arg("-H").arg("Sec-Fetch-User: ?1")
        .arg("-H").arg("Connection: keep-alive")
        .arg(url)
        .output()?;

    let status = if output.status.success() { 200 } else { 403 };
    let headers_str = String::from_utf8_lossy(&output.stderr);
    let server = headers_str.lines()
        .find(|l| l.starts_with("Server:"))
        .map(|l| l.split(": ").nth(1).unwrap_or("").to_string());

    if status == 403 {
        return Ok(PageMetadata {
            url: url.to_string(),
            is_protected: true,
            protection_reason: "HTTP 403 (curl failed)".to_string(),
            crawl_timestamp: Utc::now().timestamp(),
            ..Default::default()
        });
    }

    let html = String::from_utf8_lossy(&output.stdout).to_string();
    let html_str = html.as_str();

    println!("HTML length: {} bytes", html.len());
    if html.len() > 500 {
        println!("First 500 chars:\n{}", &html[..500]);
    }

    // --- Parse HTML ---
    let document = Html::parse_document(html_str);

    let title = document.select(&Selector::parse("title").unwrap())
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string());

    let mut meta_description = None;
    let mut canonical_url = None;
    let mut robots = None;
    let mut og = HashMap::new();

    for meta in document.select(&Selector::parse("meta").unwrap()) {
        if let Some(name) = meta.attr("name").map(|s| s.to_lowercase()) {
            match name.as_str() {
                "description" => meta_description = meta.attr("content").map(|s| s.to_string()),
                "robots" => robots = meta.attr("content").map(|s| s.to_string()),
                _ => {}
            }
        }
        if let Some(property) = meta.attr("property").map(|s| s.to_lowercase()) {
            if property.starts_with("og:") {
                if let Some(content) = meta.attr("content") {
                    og.insert(property, content.to_string());
                }
            }
        }
    }

    if let Some(link) = document.select(&Selector::parse("link[rel='canonical']").unwrap()).next() {
        canonical_url = link.attr("href").map(|s| s.to_string());
    }

    let h1 = document.select(&Selector::parse("h1").unwrap())
        .next()
        .map(|h| h.text().collect::<String>().trim().to_string());

    let lang = document.select(&Selector::parse("html").unwrap())
        .next()
        .and_then(|html| html.attr("lang"))
        .map(|s| s.to_string());

    let mut metadata = PageMetadata {
        url: url.to_string(),
        title,
        meta_description,
        canonical_url,
        robots,
        lang,
        h1,
        og_title: og.get("og:title").cloned(),
        og_description: og.get("og:description").cloned(),
        og_image: og.get("og:image").cloned(),
        og_url: og.get("og:url").cloned(),
        content_type: Some("text/html".to_string()),
        last_modified: None,
        server,
        is_protected: false,
        protection_reason: "public".to_string(),
        crawl_timestamp: Utc::now().timestamp(),
    };

    if let Some(canonical) = &metadata.canonical_url {
        if let Ok(base) = Url::parse(url) {
            if let Ok(resolved) = base.join(canonical) {
                metadata.canonical_url = Some(resolved.to_string());
            }
        }
    }

    Ok(metadata)
}