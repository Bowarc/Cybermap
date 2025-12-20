mod hashmap;
mod disk;
pub use hashmap::CacheMap;
pub use disk::DiskCache;

// TODO: An in-memory cache for a proxy *might* be a little too small (lol)
// I don't want to use a db, so I'll probably implement a simple system using files (name for key, value for content)
// until it's too slow to be effective
//
// OR implement a LRU cache system
pub trait Cache: Send + Sync + std::fmt::Debug {
    fn insert(&mut self, _key: &str, _value: &str) -> Result<(), String>;
    fn get(&self, _key: &str) -> Result<Option<String>, String>;
}
