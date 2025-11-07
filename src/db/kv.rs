use crate::db::init_db::get_kv_conn;
use r2d2::PooledConnection;
use r2d2_redis::{RedisConnectionManager, redis::Commands}; // 👈 REQUIRED for .get(), .set(), etc.
use r2d2_redis::redis::{ RedisResult, cmd, ConnectionLike};
// 

pub fn get(key: &str, mut conn: PooledConnection<RedisConnectionManager>)-> Option<String>{
    // let mut conn = get_kv_conn();

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

pub fn set(key: &str, value: &str, mut conn: PooledConnection<RedisConnectionManager>) -> bool {

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

pub fn write_to_kvrocks_stream(stream: &str, data: &str) -> RedisResult<()> {
    let mut conn = get_kv_conn();

    let id: String = cmd("XADD")
        .arg(stream)
        .arg("*")
        .arg("data")
        .arg(data)
        .query(&mut *conn)?;

    println!("✅ Wrote entry to stream `{}` with ID `{}`", stream, id);
    Ok(())
}