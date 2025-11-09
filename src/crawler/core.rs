use crate::crawler::crawl::crawl_page;
use crate::db::get_kv_conn;
use crate::db::paths;
use crate::crawler::utils;
use r2d2_redis::redis::{RedisResult, cmd, pipe, Commands};

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
                index_url(&url);
                thread::sleep(Duration::from_secs(1));
            }
            None => {
                index_url("https://google.org");
                println!("⚙️ No URLs left, sleeping...");
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
            let new_urls = utils::hash_links(&res.links, paths::CRAWL_SET_PATH)?;
            enqueue_and_mark_seen(&new_urls, &mut conn);

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

    let bloom_key = "crawl_seen"; // single Bloom filter key

    for (url, hash) in new_urls {
        // Add hash to Bloom filter
        let added: i32 = cmd("BF.ADD")
            .arg(bloom_key)
            .arg(hash)
            .query(&mut **conn)?;

        if added == 1 {
            // URL is new → enqueue for crawling
            cmd("LPUSH")
                .arg(paths::CRAWL_LIST_PATH)
                .arg(url)
                .query::<()>(&mut **conn)?;
        }
    }

    Ok(())
}

fn ensure_bloom_filter(conn: &mut r2d2::PooledConnection<RedisConnectionManager>) {
    let _: RedisResult<()> = cmd("BF.RESERVE")
        .arg("crawl_seen")   // filter key
        .arg(0.01)           // 1% false positive rate
        .arg(1_000_000)      // initial capacity
        .query(&mut **conn);
}
