pub mod crawl;
pub mod utils;
pub mod core;

pub use crawl::crawl_page;
pub use utils::clean_url;
pub use core::traverse;
