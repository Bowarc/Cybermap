use std::{
    fs::OpenOptions,
    hash::{DefaultHasher, Hasher},
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Debug)]
pub struct DiskCache {
    pub root_path: PathBuf,
}

impl super::Cache for DiskCache {
    fn insert(&mut self, key: &str, value: &str) -> Result<(), String> {
        let mut hasher = DefaultHasher::default();
        hasher.write(key.as_bytes());

        let file_name = hasher.finish();
        let file_path = self.root_path.join(file_name.to_string());

        let mut file = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(file_path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };

        if let Err(e) = file.write_all(value.as_bytes()) {
            return Err(e.to_string());
        }

        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let mut hasher = DefaultHasher::default();
        hasher.write(key.as_bytes());

        let file_name = hasher.finish();
        let file_path = self.root_path.join(file_name.to_string());

        let mut file = match OpenOptions::new().read(true).open(file_path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };

        let mut buffer = String::new();

        if let Err(e) = file.read_to_string(&mut buffer) {
            return Err(e.to_string());
        }

        Ok(Some(buffer))
    }
}
