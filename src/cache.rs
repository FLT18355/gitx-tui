use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cached<T> {
    pub fetched_at: i64,
    pub data: T,
}

impl<T> Cached<T> {
    pub fn new(data: T) -> Self {
        Self {
            fetched_at: now_epoch(),
            data,
        }
    }

    pub fn age_secs(&self) -> i64 {
        (now_epoch() - self.fetched_at).max(0)
    }

    pub fn is_fresh(&self, ttl_secs: u64) -> bool {
        self.age_secs() <= ttl_secs as i64
    }
}

#[derive(Debug, Clone)]
pub struct Cache {
    dir: PathBuf,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CacheStats {
    pub files: usize,
    pub bytes: u64,
}

impl Cache {
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        secure_dir(&dir);
        Self { dir }
    }

    pub fn load<T: DeserializeOwned>(&self, key: &str) -> Option<Cached<T>> {
        let raw = fs::read(self.file(key)).ok()?;
        serde_json::from_slice(&raw).ok()
    }

    pub fn save<T: Serialize>(&self, key: &str, data: &T) -> std::io::Result<()> {
        self.save_cached(key, &Cached::new(data))
    }

    pub fn save_cached<T: Serialize>(&self, key: &str, cached: &Cached<T>) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let raw = serde_json::to_vec(cached)
            .map_err(|err| std::io::Error::other(format!("serialize cache: {err}")))?;
        let path = self.file(key);
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, raw)?;
        secure_file(&tmp);
        fs::rename(tmp, path)
    }

    pub fn clear(&self) -> std::io::Result<()> {
        if self.dir.exists() {
            fs::remove_dir_all(&self.dir)?;
        }
        fs::create_dir_all(&self.dir)?;
        secure_dir(&self.dir);
        Ok(())
    }

    pub fn clear_prefixes(&self, prefixes: &[&str]) -> std::io::Result<CacheStats> {
        let mut removed = CacheStats::default();
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Ok(removed);
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let stem = entry.file_name();
            let stem = stem.to_string_lossy();
            if prefixes.iter().any(|prefix| stem.starts_with(prefix)) {
                let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
                if fs::remove_file(&path).is_ok() {
                    removed.files += 1;
                    removed.bytes += size;
                }
            }
        }
        Ok(removed)
    }

    pub fn stats_prefixes(&self, prefixes: &[&str]) -> CacheStats {
        let mut stats = CacheStats::default();
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return stats;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let stem = entry.file_name();
            let stem = stem.to_string_lossy();
            if !prefixes.iter().any(|prefix| stem.starts_with(prefix)) {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            stats.files += 1;
            stats.bytes += meta.len();
        }
        stats
    }

    pub fn stats(&self) -> CacheStats {
        self.stats_prefixes(&[""])
    }

    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }

    fn file(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.json"))
    }
}

pub fn secure_dir(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

fn secure_file(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

pub fn search_key(query: &str, sort: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in query.as_bytes().iter().chain(b"|").chain(sort.as_bytes()) {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("search_{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Item {
        value: String,
    }

    #[test]
    fn saves_and_loads() {
        let dir = std::env::temp_dir().join(format!("gitx-tui-cache-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = Cache::new(dir.clone());
        cache
            .save(
                "item",
                &Item {
                    value: "ok".to_string(),
                },
            )
            .unwrap();
        let loaded: Cached<Item> = cache.load("item").unwrap();
        assert_eq!(loaded.data.value, "ok");
        assert!(loaded.is_fresh(10));
        let stats = cache.stats();
        assert_eq!(stats.files, 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.join("item.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
        cache.clear().unwrap();
        assert_eq!(cache.stats().files, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn search_keys_are_stable_and_distinct() {
        let first = search_key("ratatui", "best");
        assert_eq!(first, search_key("ratatui", "best"));
        assert_ne!(first, search_key("ratatui", "stars"));
        assert_ne!(first, search_key("crossterm", "best"));
    }
}
