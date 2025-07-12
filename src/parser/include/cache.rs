use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Cached include file content with metadata
#[derive(Debug, Clone)]
pub struct CachedInclude {
    pub content: String,
    pub parsed_at: SystemTime,
    pub file_modified: SystemTime,
}

impl CachedInclude {
    pub fn new(content: String, file_modified: SystemTime) -> Self {
        Self {
            content,
            parsed_at: SystemTime::now(),
            file_modified,
        }
    }

    /// Check if this cache entry is still valid
    pub fn is_valid(&self, cache_ttl: Duration, current_file_modified: SystemTime) -> bool {
        // Check if cache hasn't expired
        let cache_age = SystemTime::now()
            .duration_since(self.parsed_at)
            .unwrap_or(cache_ttl);

        if cache_age >= cache_ttl {
            return false;
        }

        // Check if file hasn't been modified since caching
        current_file_modified <= self.file_modified
    }
}

/// Cache for include file contents
#[derive(Debug)]
pub struct IncludeCache {
    cache: HashMap<PathBuf, CachedInclude>,
    max_size: usize,
    cache_ttl: Duration,
}

impl IncludeCache {
    pub fn new(max_size: usize, cache_ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            cache_ttl,
        }
    }

    /// Get cached content if valid
    pub fn get(&self, path: &PathBuf, file_modified: SystemTime) -> Option<&str> {
        self.cache.get(path).and_then(|cached| {
            if cached.is_valid(self.cache_ttl, file_modified) {
                Some(cached.content.as_str())
            } else {
                None
            }
        })
    }

    /// Insert or update cache entry
    pub fn insert(&mut self, path: PathBuf, content: String, file_modified: SystemTime) {
        // Enforce cache size limit
        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }

        let cached = CachedInclude::new(content, file_modified);
        self.cache.insert(path, cached);
    }

    /// Remove entry from cache
    pub fn remove(&mut self, path: &PathBuf) -> Option<CachedInclude> {
        self.cache.remove(path)
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&mut self) {
        let now = SystemTime::now();
        self.cache.retain(|_, cached| {
            now.duration_since(cached.parsed_at)
                .map(|age| age < self.cache_ttl)
                .unwrap_or(false)
        });
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
            max_size: self.max_size,
            cache_ttl: self.cache_ttl,
        }
    }

    /// Evict the oldest cache entry to make room
    fn evict_oldest(&mut self) {
        if let Some((oldest_path, _)) = self
            .cache
            .iter()
            .min_by_key(|(_, cached)| cached.parsed_at)
            .map(|(path, cached)| (path.clone(), cached.clone()))
        {
            self.cache.remove(&oldest_path);
        }
    }
}

impl Default for IncludeCache {
    fn default() -> Self {
        Self::new(1000, Duration::from_secs(300)) // 1000 entries, 5 minute TTL
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_size: usize,
    pub cache_ttl: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_cached_include_validity() {
        let file_time = SystemTime::now();
        let cached = CachedInclude::new("content".to_string(), file_time);

        // Should be valid immediately
        assert!(cached.is_valid(Duration::from_secs(300), file_time));

        // Should be invalid if file was modified after caching
        let later_time = file_time + Duration::from_secs(1);
        assert!(!cached.is_valid(Duration::from_secs(300), later_time));
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = IncludeCache::new(2, Duration::from_secs(300));
        let path1 = PathBuf::from("test1.yml");
        let path2 = PathBuf::from("test2.yml");
        let file_time = SystemTime::now();

        // Insert first entry
        cache.insert(path1.clone(), "content1".to_string(), file_time);
        assert_eq!(cache.get(&path1, file_time), Some("content1"));

        // Insert second entry
        cache.insert(path2.clone(), "content2".to_string(), file_time);
        assert_eq!(cache.get(&path2, file_time), Some("content2"));
        assert_eq!(cache.stats().entries, 2);

        // Remove entry
        cache.remove(&path1);
        assert_eq!(cache.get(&path1, file_time), None);
        assert_eq!(cache.stats().entries, 1);
    }

    #[test]
    fn test_cache_size_limit() {
        let mut cache = IncludeCache::new(2, Duration::from_secs(300));
        let file_time = SystemTime::now();

        // Fill cache to capacity
        cache.insert(
            PathBuf::from("test1.yml"),
            "content1".to_string(),
            file_time,
        );
        cache.insert(
            PathBuf::from("test2.yml"),
            "content2".to_string(),
            file_time,
        );
        assert_eq!(cache.stats().entries, 2);

        // Adding third entry should evict oldest
        cache.insert(
            PathBuf::from("test3.yml"),
            "content3".to_string(),
            file_time,
        );
        assert_eq!(cache.stats().entries, 2);
    }

    #[test]
    fn test_cache_cleanup() {
        let mut cache = IncludeCache::new(10, Duration::from_millis(1));
        let file_time = SystemTime::now();

        // Insert entry that will expire quickly
        cache.insert(PathBuf::from("test.yml"), "content".to_string(), file_time);
        assert_eq!(cache.stats().entries, 1);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(2));

        // Cleanup should remove expired entry
        cache.cleanup_expired();
        assert_eq!(cache.stats().entries, 0);
    }
}
