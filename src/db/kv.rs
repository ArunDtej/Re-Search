use crate::db::init_db::get_kv_conn;
use r2d2_redis::redis::Commands; // 👈 REQUIRED for .get(), .set(), etc.


pub fn get(key: &str)-> Option<String>{
    let mut conn = get_kv_conn();

    match conn.get::<_, String>(key) {
        Ok(value) => {
            println!("🔹 {} = {}", key, value);
             Some(value)
        },
        Err(_) => {
             None
        },
    }
}

pub fn set(key: &str, value: &str) -> bool {
    let mut conn = get_kv_conn();

    match conn.set::<_, _, ()>(key, value) {
        Ok(_) => {
            println!("✅ SET {} = {}", key, value);
            true
        }
        Err(err) => {
            eprintln!("❌ Failed to set key {}: {}", key, err);
            false
        }
    }
}