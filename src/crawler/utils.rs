use url::Url;
use sha1::{Digest, Sha1};
use crate::db::paths;

use crate::db::get_kv_conn;
use r2d2_redis::redis::{ pipe, cmd, RedisResult};

pub fn clean_url(url: &str) -> Option<String> {
    let Ok(mut parsed) = Url::parse(url) else { return None };

    // Remove fragment
    parsed.set_fragment(None);

    // Remove ALL query parameters
    parsed.set_query(None);

    // Normalize path: remove duplicate slashes
    let path = parsed.path();
    let normalized = path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");
    parsed.set_path(&format!("/{}", normalized.trim_start_matches('/')));

    Some(parsed.to_string())
}

pub fn hash_links(
    links: &Vec<String>,
) -> RedisResult<Vec<(String, String)>> {
    let mut hashed_links = Vec::new();

    for link in links {
        if !link.starts_with("http") {
            continue;
        }

        let mut hasher = Sha1::new();
        hasher.update(link.as_bytes());
        let key = format!("{:x}", hasher.finalize());

        hashed_links.push((link.clone(), key));
    }

    Ok(hashed_links)
}



pub async fn back_link_score(url: &str, backlinks: &[(String, String)]) {
    let mut kv_conn = get_kv_conn();

    // These are the *Bloom filters* themselves
    let bloom_url_filter = paths::URL_SCORE;
    let bloom_domain_filter = paths::DOMAIN_SCORE;

    // These are key prefixes for counters
    let url_score_base_path = paths::URL_SCORE;      // e.g., "url_score_data"
    let domain_score_base_path = paths::DOMAIN_SCORE; // e.g., "domain_score_data"

    // ⚡ Use a pipeline for better performance
    let mut pipeline = pipe();

    for (backlink_url, _) in backlinks {
        let backlink_url = backlink_url.trim();
        if backlink_url.is_empty() {
            continue;
        }

        // =========================
        // URL-LEVEL SCORING
        // =========================
        let url_key = format!("{}:{}", url_score_base_path, backlink_url);

        // check Bloom filter before incrementing
        let exists: bool = cmd("BF.EXISTS")
            .arg(bloom_url_filter)
            .arg(backlink_url)
            .query(&mut *kv_conn)
            .unwrap_or(false);

        if !exists {
            pipeline.cmd("INCR").arg(&url_key);
            // after incrementing, add to Bloom filter
            pipeline.cmd("BF.ADD").arg(bloom_url_filter).arg(backlink_url);
        }

        // =========================
        // DOMAIN-LEVEL SCORING
        // =========================
        if let Some(domain) = Url::parse(backlink_url)
            .ok()
            .and_then(|u| u.domain().map(|d| d.to_string()))
        {
            let domain_key = format!("{}:{}", domain_score_base_path, domain);

            let domain_exists: bool = cmd("BF.EXISTS")
                .arg(bloom_domain_filter)
                .arg(&domain)
                .query(&mut *kv_conn)
                .unwrap_or(false);

            if !domain_exists {
                pipeline.cmd("INCR").arg(&domain_key);
                pipeline.cmd("BF.ADD").arg(bloom_domain_filter).arg(&domain);
            }
        }
    }

    // =========================
    // MAIN TARGET URL + DOMAIN
    // =========================
    let main_url_key = format!("{}:{}", url_score_base_path, url);
    let main_exists: bool = cmd("BF.EXISTS")
        .arg(bloom_url_filter)
        .arg(url)
        .query(&mut *kv_conn)
        .unwrap_or(false);

    if !main_exists {
        pipeline.cmd("INCR").arg(&main_url_key);
        pipeline.cmd("BF.ADD").arg(bloom_url_filter).arg(url);
    }

    if let Some(main_domain) = Url::parse(url)
        .ok()
        .and_then(|u| u.domain().map(|d| d.to_string()))
    {
        let main_domain_key = format!("{}:{}", domain_score_base_path, main_domain);

        let main_domain_exists: bool = cmd("BF.EXISTS")
            .arg(bloom_domain_filter)
            .arg(&main_domain)
            .query(&mut *kv_conn)
            .unwrap_or(false);

        if !main_domain_exists {
            pipeline.cmd("INCR").arg(&main_domain_key);
            pipeline.cmd("BF.ADD").arg(bloom_domain_filter).arg(&main_domain);
        }
    }

    // Execute all batched ops
    if let Err(err) = pipeline.query::<()>(&mut *kv_conn) {
        eprintln!("⚠️ Redis pipeline execution failed: {}", err);
    } else {
        println!("✅ Finished backlink + domain scoring for {}", url);
    }
}
