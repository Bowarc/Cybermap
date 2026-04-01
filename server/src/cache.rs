mod disk;
mod hashmap;

pub use disk::DiskCache;
pub use hashmap::CacheMap;

pub trait Cache: Send + Sync + std::fmt::Debug {
    fn insert(&mut self, _key: &str, _value: &str) -> Result<(), String>;
    fn get(&self, _key: &str) -> Result<Option<String>, String>;
}
