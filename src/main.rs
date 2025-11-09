mod db;
mod crawler;
mod common;
use std::env;
use dotenvy::dotenv;

fn main() {

    dotenv().ok();


    let url = "https://bluecloudsoftech.com";

    match crawler::crawl_page(url) {
        Ok(Some(res)) => {
            // eprintln!("Final metadata: {:#?}", res)
            println!("{}",res.links.len());
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
