use std::hash::Hasher;

use twox_hash::XxHash64;

pub fn hash64(text: &str) -> u64 {
    let mut hasher = XxHash64::with_seed(0);
    hasher.write(text.as_bytes());
    hasher.finish()
}
