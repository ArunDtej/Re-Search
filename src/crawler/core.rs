use crate::common::DOMAINS_SET;
use crate::crawler::crawl::crawl_page;
use crate::crawler::utils;
use crate::db::get_kv_conn;
use crate::db::paths;
use r2d2_redis::redis::{RedisResult, cmd, pipe};

use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

use r2d2_redis::RedisConnectionManager;

use std::time::Duration;

use tokio::task;
use tokio::time::sleep;
use serde_json::json;

pub async fn traverse() {
    let mut conn = get_kv_conn();
    ensure_bloom_filter(&mut conn); // optional (auto creates filter if missing)

    let num_tasks: u16 = 8; // lightweight async tasks

    for i in 0..num_tasks {
        task::spawn(async move {
            println!("🚀 async crawler #{}", i + 1);
            crawler_thread().await;
        });
    }

    // keep main alive
    loop {
        sleep(Duration::from_secs(3600)).await;
    }
}

async fn crawler_thread() {
    let crawl_list = paths::CRAWL_LIST_PATH;
    let mut conn: r2d2::PooledConnection<r2d2_redis::RedisConnectionManager> = get_kv_conn();

    loop {
        let url: Option<String> = cmd("RPOP")
            .arg(crawl_list)
            .query(&mut *conn)
            .unwrap_or(None);

        match url {
            Some(url) => {
                println!("Fetched url {}", url);
                let res = index_url(&url).await;
                // println!("{:?}", res);

                sleep(Duration::from_secs(1)).await;
            }
            None => {
                refill_if_empty(&mut conn).await;
                sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

async fn index_url(url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url_owned = url.to_string();
    let data: Option<super::crawl::CrawlResult> = tokio::task::spawn_blocking(move || crawl_page(&url_owned)).await??;

    // println!("Crawled data for {}, data {:?}", url, data);
    let mut conn: r2d2::PooledConnection<r2d2_redis::RedisConnectionManager> = get_kv_conn();

    match data {
        Some(res) => {
            let new_urls = utils::hash_links(&res.links)?;
            let urls_owned = new_urls.clone();    
            
            let _ = tokio::task::spawn_blocking(move || enqueue_and_mark_seen(&new_urls, &mut conn)).await;

            let docs = json!([
                {
                    "url": res.metadata.url ,
                    "title": res.metadata.title,
                    "crawl_timestamp": res.metadata.crawl_timestamp,
                    "cleaned_text": res.metadata.cleaned_text,
                    "meta_description": res.metadata.meta_description,
                    "last_modified": res.metadata.last_modified,
                    "h1": res.metadata.h1,
                }
            ]);

            ingest_to_quickwit(&docs, "http://127.0.0.1:7280/api/v1/pages/ingest").await?;

            let link_owned = url.to_string();      // Own the String
             
            tokio::task::spawn_blocking(move || {
                // ✅ Use the owned values directly — don't create new temporary strings
                utils::back_link_score(&link_owned, &urls_owned);
            })
            .await
            .expect("backlink score task failed");

        }
        None => {
            println!("⚠️ Skipped: {}", url);
        }
    };
    Ok(())
}

fn enqueue_and_mark_seen(
   new_urls: &[(String, String)], // (url, hash)
    conn: &mut r2d2::PooledConnection<RedisConnectionManager>,
) -> RedisResult<()> {
    if new_urls.is_empty() {
        return Ok(());
    }

    let bloom_key = "crawl_seen";
    let mut bloom_pipe = pipe();

    // Stage 1: build pipeline for all BF.ADD
    for (_, hash) in new_urls {
        bloom_pipe.cmd("BF.ADD").arg(bloom_key).arg(hash);
    }

    // Execute all BF.ADD at once
    let results: Vec<i32> = bloom_pipe.query(&mut **conn)?;

    // Stage 2: LPUSH only new URLs (added == 1)
    let mut push_pipe = pipe();
    let mut push_count = 0; // <-- track number of added URLs

    for ((url, _), added) in new_urls.iter().zip(results) {
        println!("Added url {}, {}", url, added);
        if added == 1 {
            push_pipe.cmd("LPUSH").arg(paths::CRAWL_LIST_PATH).arg(url);
            push_count += 1;
        }
    }

    if push_count > 0 {
        let _: RedisResult<()> = push_pipe.query(&mut **conn);
        println!("Added {} urls", push_count);
    }

    Ok(())
}

fn ensure_bloom_filter(conn: &mut r2d2::PooledConnection<RedisConnectionManager>) {
    let _: RedisResult<()> = cmd("BF.RESERVE")
        .arg(paths::CRAWL_SEEN) // filter key
        .arg(0.01) // 1% false positive rate
        .arg(1_000_000_000) // initial capacity
        .query(&mut **conn);

    let _: RedisResult<()> = cmd("BF.RESERVE")
        .arg(paths::URL_SCORE_FILTER) // filter key
        .arg(0.01) // 1% false positive rate
        .arg(1_000_000_000) // initial capacity
        .query(&mut **conn);

    let _: RedisResult<()> = cmd("BF.RESERVE")
        .arg(paths::DOMAIN_SCORE_FILTER) // filter key
        .arg(0.01) // 1% false positive rate
        .arg(100_000_000) // initial capacity
        .query(&mut **conn);

    let _: RedisResult<()> = cmd("EXPIRE")
        .arg(paths::CRAWL_SEEN)
        .arg(60 * 60 * 24 * 31) // seconds
        .query(&mut **conn);

    println!("Creating KV Filters");
}

async fn refill_if_empty(conn: &mut r2d2_redis::redis::Connection) {
    let len: i64 = cmd("LLEN")
        .arg(paths::CRAWL_LIST_PATH)
        .query(conn)
        .unwrap_or(0);

    if len == 0 {
        // Try acquiring lock (SETNX returns 1 if lock acquired, 0 if already set)
        let got_lock: bool = cmd("SETNX")
            .arg("crawler:refill_lock")
            .arg(1)
            .query(conn)
            .unwrap_or(false);

        if got_lock {
            println!("⚙️ Acquired lock — refilling from DOMAINS_SET...");

            // Optional: auto-expire lock after 30s to avoid deadlocks
            let _: RedisResult<()> = cmd("EXPIRE").arg("crawler:refill_lock").arg(30).query(conn);

            // Perform refill
            let domains = DOMAINS_SET;
            for domain in &domains {
                let full_url = format!("https://{}", domain);
                let _: RedisResult<()> = cmd("LPUSH")
                    .arg(paths::CRAWL_LIST_PATH)
                    .arg(&full_url)
                    .query(conn);
            }

            println!("✅ Refilled {} domains into crawl list", domains.len());

            // Release lock
            let _: RedisResult<()> = cmd("DEL").arg("crawler:refill_lock").query(conn);
        } else {
            println!("🕓 Another thread is already refilling, waiting...");
        }
    } else {
        println!("🕓 Queue not empty (len={})", len);
    }

    sleep(Duration::from_secs(10)).await;
}

pub async fn ingest_to_quickwit(data: &Value, endpoint: &str) -> Result<()> {
    let client = Client::new();
    let mut ndjson = String::new();

    match data {
        Value::Array(arr) => {
            for item in arr {
                ndjson.push_str(&serde_json::to_string(item)?);
                ndjson.push('\n');
            }
        }
        Value::Object(_) => {
            ndjson.push_str(&serde_json::to_string(data)?);
            ndjson.push('\n');
        }
        _ => anyhow::bail!("Input must be a JSON object or array of objects"),
    }

    let resp = client
        .post(endpoint)
        .header("Content-Type", "application/x-ndjson")
        .body(ndjson)
        .send()
        .await?;

    println!("✅ Status: {}", resp.status());
    println!("🔹 Response: {}", resp.text().await?);
    Ok(())
}
