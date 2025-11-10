use url::Url;
use r2d2_redis::redis::{ RedisResult};
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

pub fn hash_links(
    links: &Vec<String>,
) -> RedisResult<Vec<(String, String)>> {
    let mut hashed_links = Vec::new();

    for link in links {
        if !link.starts_with("http") {
            continue;
        }

        let mut hasher = Sha1::new();
        hasher.update(link.as_bytes());
        let key = format!("{:x}", hasher.finalize());

        hashed_links.push((link.clone(), key));
    }

    Ok(hashed_links)
}
