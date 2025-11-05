mod db;
mod crawler;
fn main() {

    db::connect_db();
    db::init_db::connect_db();

    let metadata =  crawler::fetch_metadata("https://econpapers.repec.org/paper/eugwpaper/ki-01-25-150-en-n.htm");

    eprintln!("Final metadata: {:#?}", metadata);
    
    println!("Hello, world!");
}
