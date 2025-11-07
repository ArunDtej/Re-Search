use url::Url;


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