// cl -> crawl_list
// uscr -> url_score
// dscr -> domain_score
pub const CRAWL_LIST_PATH: &str = "cl"; // to be crawled lpush rpop 
pub const URL_SCORE: &str = "cs"; // already crawled, avoid for one month
