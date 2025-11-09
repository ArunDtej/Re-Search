use crate::crawler::crawl::crawl_page;
use crate::db::get_kv_conn;
use crate::db::paths;
use crate::crawler::utils;
use r2d2_redis::redis::{RedisResult, cmd};
use std::thread;
use std::time::Duration;

fn traverse() {}


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
                thread::sleep(Duration::from_secs(1));
            }
            None => {
                println!("⚙️ No URLs left, sleeping...");
                thread::sleep(Duration::from_secs(60));
            }
        }
    }
}

fn index_url(url: &str) {
    let data = crawl_page(url);

    let mut conn: r2d2::PooledConnection<r2d2_redis::RedisConnectionManager> = get_kv_conn();

    match data {
        Ok(Some(res)) => {
            let new_urls = utils::find_unseen_links(&res.links, paths::CRAWL_SET_PATH, &mut conn);
        }
        Ok(None) => {
            println!("⚠️ Skipped: {}", url);
        }
        Err(e) => {
            eprintln!("❌ Error: {} -> {}", url, e);
        }
    };
}
