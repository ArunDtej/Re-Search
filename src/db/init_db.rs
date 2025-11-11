use std::env;
use r2d2_redis::{r2d2, RedisConnectionManager};
use once_cell::sync::Lazy;


pub static KVPOOL: Lazy<r2d2::Pool<RedisConnectionManager>> = Lazy::new(|| {
    let database_url: String  = env::var("ROCKS_STR").unwrap_or("6666".into()).parse().unwrap();

    let manager = RedisConnectionManager::new(database_url)
        .expect("Invalid Redis URL");
    r2d2::Pool::builder()
        .max_size(200)
        .build(manager)
        .expect("Failed to create pool")
});

pub fn get_kv_conn() -> r2d2::PooledConnection<RedisConnectionManager> {
    KVPOOL.get().expect("Failed to get connection from pool")
}