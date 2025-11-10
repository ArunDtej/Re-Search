use crate::crawler::crawl::crawl_page;
use crate::db::get_kv_conn;
use crate::db::paths;
use crate::crawler::utils;
use r2d2_redis::redis::{RedisResult, cmd, pipe};
use crate::common::DOMAINS_SET;

// use redis::{cmd, pipe, Commands, RedisResult};
use r2d2_redis::RedisConnectionManager;

use std::thread;
use std::time::Duration;

pub fn traverse() {
    let mut conn = get_kv_conn();
    ensure_bloom_filter(&mut conn);  // optional (auto creates filter if missing)
    crawler_thread()
}


fn crawler_thread() {
    let crawl_list = paths::CRAWL_LIST_PATH;
    let mut conn: r2d2::PooledConnection<r2d2_redis::RedisConnectionManager> = get_kv_conn();

    loop {
        let url: Option<String> = cmd("RPOP")
            .arg(crawl_list)
            .query(&mut *conn)
            .unwrap_or(None);

        match url {
            Some(url) => {
                let _ = index_url(&url);
                thread::sleep(Duration::from_secs(1));
            }
            None => {

                println!("⚙️ No URLs left — refilling from DOMAINS_SET...");

                let domains = DOMAINS_SET;
                for domain in &domains {
                    let full_url = format!("https://{}", domain);

                    let _: RedisResult<()> = cmd("LPUSH")
                        .arg(paths::CRAWL_LIST_PATH)
                        .arg(&full_url)
                        .query(&mut *conn);
                }

                println!("✅ Refilled {} domains into crawl list", domains.len());
                thread::sleep(Duration::from_secs(10));
            }
        }
    }
}

fn index_url(url: &str) -> Result<(), Box<dyn std::error::Error>>  {
    let data = crawl_page(url);

    let mut conn: r2d2::PooledConnection<r2d2_redis::RedisConnectionManager> = get_kv_conn();

    match data {
        Ok(Some(res)) => {
            let new_urls = utils::hash_links(&res.links)?;
            let _ = enqueue_and_mark_seen(&new_urls, &mut conn);

            println!("{} {}", url,new_urls.len());
        }
        Ok(None) => {
            println!("⚠️ Skipped: {}", url);
        }
        Err(e) => {
            eprintln!("❌ Error: {} -> {}", url, e);
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
        if added == 1 {
            push_pipe.cmd("LPUSH").arg(paths::CRAWL_LIST_PATH).arg(url);
            push_count += 1;
        }
    }

    if push_count > 0 {
        let _: RedisResult<()> = push_pipe.query(&mut **conn);
    }

    Ok(())
}


fn ensure_bloom_filter(conn: &mut r2d2::PooledConnection<RedisConnectionManager>) {
    let _: RedisResult<()> = cmd("BF.RESERVE")
        .arg("crawl_seen")   // filter key
        .arg(0.01)           // 1% false positive rate
        .arg(1_000_000)      // initial capacity
        .query(&mut **conn);

    let _: RedisResult<()> = cmd("EXPIRE") 
        .arg("crawl_seen")
        .arg(60 * 60 * 24 * 31) // seconds 
        .query(&mut **conn);

}
