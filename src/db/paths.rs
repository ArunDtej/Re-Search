// Key paths
// cl -> crawl_list
// uscr -> url_score
// dscr -> domain_score
pub const CRAWL_LIST_PATH: &str = "cl"; // to be crawled lpush rpop 
pub const URL_SCORE: &str = "cs"; // Track url backlink score
pub const DOMAIN_SCORE: &str = "dscr"; // Track domain backlink score

// Filters
pub const CRAWL_SEEN: &str = "crawl_seen"; // Track recently crawled
pub const URL_SCORE_FILTER: &str = "url_score"; // Track URL backlink score
pub const DOMAIN_SCORE_FILTER: &str = "domain_score"; // Track Domain backlink score