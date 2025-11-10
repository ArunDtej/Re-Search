use chrono::Utc;
use curl::easy::{Easy, List};
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use url::Url;

use crate::common::utils::random_ua;
use crate::crawler::clean_url;

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
    pub cleaned_text: Option<String>, // ✅ new field
}

#[derive(Debug, Clone)]
pub struct CrawlResult {
    pub metadata: PageMetadata,
    pub links: Vec<String>,
}

pub fn crawl_page(url: &str) ->Result<Option<CrawlResult>, Box<dyn std::error::Error + Send + Sync>> {
    let mut easy = Easy::new();
    easy.url(url)?;
    easy.follow_location(true)?;
    easy.max_redirections(10)?;
    easy.timeout(std::time::Duration::from_secs(30))?;
    easy.accept_encoding("gzip, deflate")?;
    easy.useragent(&random_ua())?;

    // Headers
    let mut headers = List::new();
    headers.append("Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")?;
    headers.append("Accept-Language: en-US,en;q=0.9")?;
    headers.append("Connection: keep-alive")?;
    easy.http_headers(headers)?;

    // Buffers
    let mut html_bytes = Vec::new();
    let mut response_headers = Vec::new();
    let (mut status_code, mut content_type, mut server, mut last_modified) =
        (0, None, None, None);

    // === Perform request ===
    {
        let mut transfer = easy.transfer();
        transfer.header_function(|h| {
            response_headers.push(String::from_utf8_lossy(h).to_string());
            true
        })?;
        transfer.write_function(|d| {
            html_bytes.extend_from_slice(d);
            Ok(d.len())
        })?;
        transfer.perform()?;
    }

    status_code = easy.response_code()? as i32;
    content_type = easy.content_type()?.map(|s| s.to_string());

    for line in &response_headers {
        let line_lower = line.trim().to_lowercase();
        if line_lower.starts_with("server:") {
            server = line_lower.strip_prefix("server:").map(|s| s.trim().to_string());
        } else if line_lower.starts_with("last-modified:") {
            last_modified = line_lower
                .strip_prefix("last-modified:")
                .map(|s| s.trim().to_string());
        }
    }

    // === Skip non-200 ===
    if status_code != 200 {
        println!("[SKIP] {} -> HTTP {}", url, status_code);
        return Ok(None);
    }

    // === Non-HTML ===
    let is_html = content_type
        .as_deref()
        .map(|ct| ct.contains("text/html"))
        .unwrap_or(false);
    if !is_html {
        println!("[SKIP] {} -> Non-HTML ({:?})", url, content_type);
        return Ok(None);
    }

    // === Parse HTML ===
    let html = String::from_utf8_lossy(&html_bytes).to_string();
    let document = Html::parse_document(&html);

    let title = document
        .select(&Selector::parse("title").unwrap())
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

    if let Some(link) = document
        .select(&Selector::parse("link[rel='canonical']").unwrap())
        .next()
    {
        canonical_url = link.attr("href").map(|s| s.to_string());
    }

    let h1 = document
        .select(&Selector::parse("h1").unwrap())
        .next()
        .map(|h| h.text().collect::<String>().trim().to_string());

    let lang = document
        .select(&Selector::parse("html").unwrap())
        .next()
        .and_then(|html| html.attr("lang"))
        .map(|s| s.to_string());

    // === Extract links ===
    let mut links_set = HashSet::new();
    let base_url = Url::parse(url).ok();

    for elem in document.select(&Selector::parse("a[href]").unwrap()) {
        let Some(href) = elem.attr("href") else { continue; };
        let rel = elem.attr("rel").unwrap_or("");
        if href.starts_with("javascript:")
            || href.starts_with("mailto:")
            || href.starts_with("tel:")
            || rel.contains("nofollow")
            || rel.contains("noopener")
            || rel.contains("noreferrer")
            || rel.contains("ugc")
            || rel.contains("sponsored")
        {
            continue;
        }
        if let Some(resolved) = resolve_url(url, href, &base_url) {
            links_set.insert(resolved);
        }
    }

    let mut cleaned_links = HashSet::new();
    for raw_url in links_set {
        if let Some(cleaned) = clean_url(&raw_url) {
            cleaned_links.insert(cleaned);
        }
    }
    let links: Vec<String> = cleaned_links.into_iter().collect();

    // === Cleaned Text Extraction ===
    let cleaned_text = extract_clean_text(&html);

    let mut metadata = PageMetadata {
        url: url.to_string(),
        title,
        meta_description,
        canonical_url: canonical_url.clone(),
        robots,
        lang,
        h1,
        og_title: og.get("og:title").cloned(),
        og_description: og.get("og:description").cloned(),
        og_image: og.get("og:image").cloned(),
        og_url: og.get("og:url").cloned(),
        content_type,
        last_modified,
        server,
        is_protected: false,
        protection_reason: "public".to_string(),
        crawl_timestamp: Utc::now().timestamp(),
        cleaned_text: Some(cleaned_text), // ✅ include text
        ..Default::default()
    };

    // Resolve canonical
    if let Some(canonical) = &metadata.canonical_url {
        if let Ok(base) = Url::parse(url) {
            if let Ok(resolved) = base.join(canonical) {
                metadata.canonical_url = Some(resolved.to_string());
            }
        }
    }

    Ok(Some(CrawlResult { metadata, links }))
}

fn resolve_url(base_str: &str, href: &str, fallback_base: &Option<Url>) -> Option<String> {
    let trimmed = href.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("javascript:")
        || trimmed.starts_with("mailto:")
    {
        return None;
    }
    if let Ok(url) = Url::parse(trimmed) {
        return Some(url.to_string());
    }
    if let Ok(base) = Url::parse(base_str) {
        if let Ok(joined) = base.join(trimmed) {
            return Some(joined.to_string());
        }
    }
    if let Some(base) = fallback_base {
        if let Ok(joined) = base.join(trimmed) {
            return Some(joined.to_string());
        }
    }
    None
}

// use scraper::{Html, Selector};

/// Extracts visible, human-readable text (no HTML, no CSS, no JS)
fn extract_clean_text(html: &str) -> String {
    // Parse the document
    let document = Html::parse_document(html);

    // Remove all <script>, <style>, <noscript>, <iframe>, <svg>, etc. first
    let skip_tags = [
        "script",
        "style",
        "noscript",
        "iframe",
        "canvas",
        "svg",
        "meta",
        "link",
        "button",
        "input",
        "form",
        "nav",
        "footer",
        "header",
    ];

    // Collect text only from content-bearing elements
    let mut text = String::new();
    for selector in ["p", "article", "section", "main", "div", "span", "li"].iter() {
        let sel = Selector::parse(selector).unwrap();
        for elem in document.select(&sel) {
            // Skip if this element (or its parent) is inside one of the skip tags
            let mut inside_skipped = false;
            let mut parent = elem.parent();
            while let Some(p) = parent {
                if let Some(el) = p.value().as_element() {
                    if skip_tags.contains(&el.name()) {
                        inside_skipped = true;
                        break;
                    }
                }
                parent = p.parent();
            }
            if inside_skipped {
                continue;
            }

            // Extract visible text nodes
            for txt in elem.text() {
                let t = txt.trim();
                if !t.is_empty()
                    && !t.starts_with('{')
                    && !t.ends_with('}')
                    && !t.contains("var(")
                    && !t.contains(':') // filter out CSS rules
                    && !t.contains(';')
                {
                    text.push_str(t);
                    text.push(' ');
                }
            }
        }
    }

    // Normalize whitespace
    let cleaned = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    // Limit to prevent enormous pages
    cleaned.chars().take(8000).collect()
}
