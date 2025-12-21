use std::collections::HashMap;

pub type CacheMap = HashMap<Box<str>, Box<str>>;

impl super::Cache for CacheMap {
    fn insert(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.insert(
            key.to_owned().into_boxed_str(),
            value.to_owned().into_boxed_str(),
        );
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.get(key).map(|v| v.as_ref().to_owned()))
    }
}
