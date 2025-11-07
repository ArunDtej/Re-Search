mod db;
mod crawler;
mod common;
fn main() {

    db::connect_db();
    db::init_db::connect_db();

    // let metadata =  crawler::crawl_page("https://search.app.goo.gl/");

    let url = "https://google.com";

    match crawler::crawl_page(url) {
        Ok(Some(res)) => eprintln!("Final metadata: {:#?}", res),
        Ok(None) => {
            println!("⚠️ Skipped: {}", url);
        }
        Err(e) => {
            eprintln!("❌ Error: {} -> {}", url, e);
        }  
    };
    
    println!("Hello, world!");
}
