use std::path::PathBuf;

pub struct ParseCache {
    #[allow(dead_code)]
    cache_dir: PathBuf,
}

impl ParseCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    pub async fn get<T>(&self, _key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // TODO: Implement cache retrieval
        None
    }

    pub async fn set<T>(&self, _key: &str, _value: &T) -> Result<(), std::io::Error>
    where
        T: serde::Serialize,
    {
        // TODO: Implement cache storage
        Ok(())
    }
}
