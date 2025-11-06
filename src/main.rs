mod db;
mod crawler;
mod common;
fn main() {

    db::connect_db();
    db::init_db::connect_db();

    let metadata =  crawler::fetch_metadata("https://ecpapers.reec.org/paper/eugwpaper/ki-01-25-150-en-n.htm");

    eprintln!("Final metadata: {:#?}", metadata);

    println!("{:#?}",  common::config::UA);
    
    println!("Hello, world!");
}
