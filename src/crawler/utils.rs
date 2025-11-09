use url::Url;
use r2d2_redis::redis::{cmd, RedisResult};
use sha1::{Digest, Sha1};


pub fn clean_url(url: &str) -> Option<String> {
    let Ok(mut parsed) = Url::parse(url) else { return None };

    // Remove fragment
    parsed.set_fragment(None);

    // Remove ALL query parameters
    parsed.set_query(None);

    // Normalize path: remove duplicate slashes
    let path = parsed.path();
    let normalized = path
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");
    parsed.set_path(&format!("/{}", normalized.trim_start_matches('/')));

    Some(parsed.to_string())
}


pub fn find_unseen_links(
    links: &Vec<String>,
    crawl_set: &str,
    conn: &mut r2d2::PooledConnection<r2d2_redis::RedisConnectionManager>,
) -> RedisResult<Vec<(String, String)>> {

    let mut pipeline = r2d2_redis::redis::pipe();
    let mut hashed_links = Vec::new();


    for link in links {

        if !link.starts_with("http") {
            continue;
        }


        let mut hasher = Sha1::new();
        hasher.update(link.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        let key = format!("{}:{}", crawl_set, hash);


        pipeline.cmd("EXISTS").arg(&key);
        hashed_links.push((link.clone(), key));
    }

    let results: Vec<i32> = pipeline.query(&mut **conn)?;

    let unseen: Vec<(String, String)> = hashed_links
        .into_iter()
        .zip(results)
        .filter_map(|((link, key), exists)| {
            if exists == 0 {
                Some((link, key))
            } else {
                None
            }
        })
        .collect();

    Ok(unseen)
}
