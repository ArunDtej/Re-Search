mod db;
mod crawler;
mod common;
use std::env;
use dotenvy::dotenv;

use crate::db::write_to_kvrocks_stream;

fn main() {

    dotenv().ok();


    let url = "https://bluecloudsoftech.com";

    match crawler::crawl_page(url) {
        Ok(Some(res)) => {
            // eprintln!("Final metadata: {:#?}", res)
            write_to_kvrocks_stream("stream", "data");
        },
        Ok(None) => {
            println!("⚠️ Skipped: {}", url);
        }
        Err(e) => {
            eprintln!("❌ Error: {} -> {}", url, e);
        }  
    };
    
    println!("Hello, world!");
}
