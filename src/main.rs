mod db;
mod crawler;
mod common;
use dotenvy::dotenv;

fn main() {

    dotenv().ok();

    crawler::core::traverse();
    println!("Hello, world!");
}
