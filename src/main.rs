mod db;
mod crawler;
mod common;
use dotenvy::dotenv;
use tokio::runtime::Builder;


fn main() {

    dotenv().ok();

    let runtime = Builder::new_multi_thread()
        .worker_threads(32)
        .max_blocking_threads(512)
        .enable_all() // enables time, I/O, etc.
        .build()
        .unwrap();

    runtime.block_on(async {
        println!("✅ Tokio runtime ready with 32 async threads + 512 blocking threads");

        crawler::core::traverse().await;
    });

    // crawler::core::traverse();
    println!("Hello, world!");
}
