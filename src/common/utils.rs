use crate::common::UA;
use rand::prelude::*;


pub fn random_ua() -> &'static str {
    let mut rng = rand::thread_rng();
    UA.choose(&mut rng).unwrap()
}